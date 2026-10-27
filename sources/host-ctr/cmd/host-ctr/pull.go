// host-ctr pulls images via the containerd transfer service: the daemon runs the
// digestion verifier and fetches from the registry, with credentials supplied via
// a registry.CredentialHelper.

package main

import (
	"context"
	"encoding/base64"
	"fmt"
	"math/rand"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/containerd/containerd/v2/client"
	"github.com/containerd/containerd/v2/core/remotes/docker"
	"github.com/containerd/containerd/v2/core/transfer"
	timage "github.com/containerd/containerd/v2/core/transfer/image"
	tregistry "github.com/containerd/containerd/v2/core/transfer/registry"
	"github.com/containerd/containerd/v2/defaults"
	"github.com/containerd/log"
	"github.com/containerd/platforms"
	"github.com/urfave/cli/v2"
)

const hostDigestionTrustPolicyDir = "/etc/host-containerd/image-verifiers/digestion/trustpolicy.d"

// containerdCertsDir is the hosts.d dir thar-be-registries renders from
// settings.container-registry.mirrors; used to resolve registry mirrors.
const containerdCertsDir = "/etc/containerd/certs.d"

// extraPullFlags adds the --expected-digest flag.
func extraPullFlags() []cli.Flag {
	return []cli.Flag{
		&cli.StringFlag{
			Name:  "expected-digest",
			Usage: "pin the pulled image to this `sha256:...` manifest digest; the daemon's digestion verifier enforces it during the transfer-service pull",
		},
	}
}

// expectedDigest returns the value of --expected-digest (empty if unset).
func expectedDigest(c *cli.Context) string { return c.String("expected-digest") }

// pullImage pulls an image via the containerd transfer service, writing a
// per-image digestion trust policy drop-in first when expectedDigest is set.
func pullImage(ctx context.Context, source string, c *client.Client, registryConfigPath string, labels map[string]string, expectedDigest string) (client.Image, error) {
	var registryConfig *RegistryConfig
	if registryConfigPath != "" {
		var err error
		registryConfig, err = NewRegistryConfig(registryConfigPath)
		if err != nil {
			log.G(ctx).
				WithError(err).
				WithField("registry-config", registryConfigPath).
				Error("failed to read registry config")
			return nil, err
		}
	} else {
		registryConfig = NewBlankRegistryConfig()
	}

	// Clear any stale drop-in for this ref left by a killed host-ctr, so an
	// unpinned pull is never checked against a stale digest.
	removeHostDigestionDropin(ctx, source)

	if expectedDigest != "" {
		if pinned := sourcePinnedDigest(source); pinned != "" && pinned != expectedDigest {
			return nil, fmt.Errorf("conflicting digest pins for %s: ref pins %s but --expected-digest is %s",
				source, pinned, expectedDigest)
		}
		if err := writeHostDigestionDropin(source, expectedDigest); err != nil {
			return nil, fmt.Errorf("writing digestion trust policy drop-in: %w", err)
		}
		defer removeHostDigestionDropin(ctx, source)
	}

	// Retry transient failures so one blip doesn't fail an essential container's boot.
	const maxRetryAttempts = 5
	const intervalMultiplier = 2
	const maxRetryInterval = 30 * time.Second
	const jitterPeakAmplitude = 4000
	const jitterLowerBound = 2000
	retryInterval := 1 * time.Second
	retryAttempts := 0
	for {
		img, err := pullViaTransferService(ctx, source, c, registryConfig, labels)
		if err == nil {
			return img, nil
		}
		// A digestion-verifier block is deterministic; retrying cannot change it.
		if isVerifierRejection(err) {
			return nil, err
		}
		if retryAttempts >= maxRetryAttempts {
			if expectedDigest != "" {
				return nil, fmt.Errorf("verified pull of %s (expected %s) failed after %d retries: %w",
					source, expectedDigest, maxRetryAttempts, err)
			}
			return nil, fmt.Errorf("pull of %s failed after %d retries: %w", source, maxRetryAttempts, err)
		}
		retryIntervalWithJitter := retryInterval +
			time.Duration(rand.Int31n(jitterPeakAmplitude))*time.Millisecond +
			jitterLowerBound*time.Millisecond
		log.G(ctx).WithError(err).Warnf("failed to pull image. waiting %s before retrying...", retryIntervalWithJitter)
		timer := time.NewTimer(retryIntervalWithJitter)
		select {
		case <-timer.C:
			retryInterval *= intervalMultiplier
			if retryInterval > maxRetryInterval {
				retryInterval = maxRetryInterval
			}
			retryAttempts++
		case <-ctx.Done():
			timer.Stop()
			return nil, fmt.Errorf("context ended while retrying pull of %s: %w", source, err)
		}
	}
}

// verifyExpectedDigest fails closed unless img's manifest digest matches
// --expected-digest. No-op when no digest is pinned.
func verifyExpectedDigest(ctx context.Context, img client.Image, expectedDigest string) error {
	if expectedDigest == "" {
		return nil
	}
	got := img.Target().Digest.String()
	if err := digestMismatch(img.Name(), got, expectedDigest); err != nil {
		return err
	}
	log.G(ctx).WithField("ref", img.Name()).WithField("digest", got).Debug("image digest matches --expected-digest")
	return nil
}

// verifySourcePinnedDigest fails closed unless img resolves to exactly the
// digest its source reference pins (repo@<digest>), for any digest algorithm.
// No-op when the ref names no digest.
func verifySourcePinnedDigest(source string, img client.Image) error {
	return sourcePinMismatch(source, img.Target().Digest.String())
}

// sourcePinMismatch returns an error, failing closed, unless got matches the
// digest source pins. No-op when source names no digest.
func sourcePinMismatch(source, got string) error {
	pinned := sourcePinnedDigest(source)
	if pinned == "" {
		return nil
	}
	if got != pinned {
		return fmt.Errorf("image %q resolved to %s, not the digest %s it names", source, got, pinned)
	}
	return nil
}

// digestMismatch returns an error, failing closed, when got != want.
func digestMismatch(name, got, want string) error {
	if got == want {
		return nil
	}
	return fmt.Errorf("image %s manifest digest %s does not match --expected-digest %s", name, got, want)
}

// isVerifierRejection reports whether err is a deterministic image-verifier
// block from the transfer service, which retrying would not resolve.
func isVerifierRejection(err error) bool {
	s := err.Error()
	return strings.Contains(s, "blocked pull") || strings.Contains(s, "blocking pull")
}

// sourcePinnedDigest returns the digest a ref is pinned to (repo@sha256:...), or
// "" if the ref carries no digest.
func sourcePinnedDigest(source string) string {
	if i := strings.LastIndex(source, "@"); i >= 0 {
		return source[i+1:]
	}
	return ""
}

// isValidSha256Digest reports whether d is a well-formed "sha256:<64 hex>" digest.
func isValidSha256Digest(d string) bool {
	hex, ok := strings.CutPrefix(d, "sha256:")
	if !ok || len(hex) != 64 {
		return false
	}
	for _, c := range hex {
		if (c < '0' || c > '9') && (c < 'a' || c > 'f') {
			return false
		}
	}
	return true
}

// pullViaTransferService performs the daemon-side transfer-service pull: an
// OCIRegistry source (with host-ctr credentials supplied via a CredentialHelper)
// into the daemon's image store, unpacked for the default snapshotter.
func pullViaTransferService(ctx context.Context, source string, c *client.Client, registryConfig *RegistryConfig, labels map[string]string) (client.Image, error) {
	plat := platforms.DefaultSpec()

	reg, err := tregistry.NewOCIRegistry(ctx, source,
		tregistry.WithCredentials(&hostCtrCredentialHelper{registryConfig: registryConfig}),
		tregistry.WithHostDir(containerdCertsDir),
	)
	if err != nil {
		return nil, err
	}

	// Destination: the daemon's image store, unpacked for the default snapshotter.
	storeOpts := []timage.StoreOpt{
		timage.WithPlatforms(plat),
		timage.WithUnpack(plat, defaults.DefaultSnapshotter),
		timage.WithAllMetadata,
	}
	if len(labels) != 0 {
		storeOpts = append(storeOpts, timage.WithImageLabels(labels))
	}
	is := timage.NewStore(source, storeOpts...)

	if err := c.Transfer(ctx, reg, is, transfer.WithProgress(func(transfer.Progress) {})); err != nil {
		return nil, err
	}
	log.G(ctx).WithField("ref", source).Info("pulled via transfer service")

	return c.GetImage(ctx, source)
}

// hostCtrCredentialHelper adapts host-ctr's ECR/registry-config credential logic
// to the transfer service's registry.CredentialHelper interface.
type hostCtrCredentialHelper struct {
	registryConfig *RegistryConfig
}

// GetCredentials resolves credentials for host: ECR private, ECR public (with
// anonymous fallback), then the registry config's static credentials.
func (h *hostCtrCredentialHelper) GetCredentials(ctx context.Context, _ string, host string) (tregistry.Credentials, error) {
	// ECR private registries require authentication.
	if ecrPrivateHostPattern.MatchString(host) {
		parsed, err := parseECRHost(host)
		if err != nil {
			return tregistry.Credentials{}, fmt.Errorf("failed to parse ECR host %q: %w", host, err)
		}
		username, password, err := getECRPrivateCredentials(ctx, parsed.Region, parsed.Fips)
		if err != nil {
			return tregistry.Credentials{}, fmt.Errorf("failed to get private ECR credentials for region %s: %w", parsed.Region, err)
		}
		log.G(ctx).WithField("host", host).WithField("region", parsed.Region).Info("supplying private ECR credentials to transfer service")
		return tregistry.Credentials{Host: host, Username: username, Secret: password}, nil
	}

	// ECR public supports anonymous access; prefer configured creds, then a
	// fetched token, then fall back to anonymous.
	if host == ecrPublicHost {
		if cred, ok := h.lookupConfigCredential(host); ok {
			username, secret, err := parseAuth(cred)
			if err != nil {
				return tregistry.Credentials{}, err
			}
			return tregistry.Credentials{Host: host, Username: username, Secret: secret}, nil
		}
		username, password, err := getECRPublicCredentials(ctx)
		if err != nil {
			log.G(ctx).WithError(err).Warn("ecr-public: failed to get credentials, falling back to anonymous pull")
			return tregistry.Credentials{Host: host}, nil
		}
		return tregistry.Credentials{Host: host, Username: username, Secret: password}, nil
	}

	// Static credentials from the registry config, if any.
	if cred, ok := h.lookupConfigCredential(host); ok {
		username, secret, err := parseAuth(cred)
		if err != nil {
			return tregistry.Credentials{}, err
		}
		return tregistry.Credentials{Host: host, Username: username, Secret: secret}, nil
	}

	// No credentials: anonymous pull.
	return tregistry.Credentials{Host: host}, nil
}

// lookupConfigCredential finds a registry-config credential for host, trying the
// host verbatim then its docker default-host mapping (docker.io ->
// registry-1.docker.io).
func (h *hostCtrCredentialHelper) lookupConfigCredential(host string) (Credential, bool) {
	if h.registryConfig == nil {
		return Credential{}, false
	}
	if cred, ok := h.registryConfig.Credentials[host]; ok {
		return cred, true
	}
	for configuredHost, cred := range h.registryConfig.Credentials {
		if dh, err := docker.DefaultHost(configuredHost); err == nil && dh == host {
			return cred, true
		}
	}
	return Credential{}, false
}

// dropinPath maps an image reference to its drop-in trust policy path, keyed by
// base64url so the verifier can recompute it from containerd's `-name`.
func dropinPath(reference string) string {
	key := base64.RawURLEncoding.EncodeToString([]byte(reference))
	return filepath.Join(hostDigestionTrustPolicyDir, key+".json")
}

// writeHostDigestionDropin writes this image's digestion trust policy atomically.
func writeHostDigestionDropin(source, digest string) error {
	// digest is validated once in fetchImage before any path is taken.
	if err := os.MkdirAll(hostDigestionTrustPolicyDir, 0o700); err != nil {
		return err
	}
	data := []byte(fmt.Sprintf(`{"version": "1.0", "trustedDigests": [%q]}`+"\n", digest))
	path := dropinPath(source)
	tmp := path + ".tmp" // not *.json, so the verifier never reads a partial file
	if err := os.WriteFile(tmp, data, 0o600); err != nil {
		return err
	}
	return os.Rename(tmp, path)
}

// removeHostDigestionDropin deletes only this image's drop-in, so parallel pulls
// of other images never remove each other's policy. A missing file is fine.
func removeHostDigestionDropin(ctx context.Context, source string) {
	path := dropinPath(source)
	if err := os.Remove(path); err != nil && !os.IsNotExist(err) {
		log.G(ctx).WithError(err).Warnf("failed to remove %s", path)
	}
}
