// host-ctr pulls images via the containerd transfer service: the daemon runs the
// digestion verifier and fetches from the registry, with credentials supplied via
// a registry.CredentialHelper.

package main

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"io/fs"
	"math/rand"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"syscall"
	"time"

	containerd "github.com/containerd/containerd/v2/client"
	"github.com/containerd/containerd/v2/core/remotes/docker"
	"github.com/containerd/containerd/v2/core/transfer"
	timage "github.com/containerd/containerd/v2/core/transfer/image"
	tregistry "github.com/containerd/containerd/v2/core/transfer/registry"
	"github.com/containerd/containerd/v2/defaults"
	"github.com/containerd/log"
	"github.com/containerd/platforms"
	"github.com/opencontainers/go-digest"
	"github.com/urfave/cli/v2"
)

// hostDigestionTrustPolicyDir holds the per-image trust policy drop-ins.
const hostDigestionTrustPolicyDir = "/etc/host-containerd/image-verifiers/digestion/trustpolicy.d"

// extraPullFlags adds the --expected-digest flag.
func extraPullFlags() []cli.Flag {
	return []cli.Flag{
		&cli.StringFlag{
			Name:  "expected-digest",
			Usage: "pin the pulled image to this `sha256:...` digest (the index digest for a multi-arch image); the daemon's digestion verifier enforces it during the transfer-service pull",
		},
	}
}

// expectedDigest returns the value of --expected-digest (empty if unset).
func expectedDigest(c *cli.Context) string { return c.String("expected-digest") }

// pullImage pulls an image via the containerd transfer service, writing a
// per-image digestion trust policy drop-in first when expectedDigest is set.
func pullImage(ctx context.Context, source string, client *containerd.Client, registryConfigPath string, labels map[string]string, expectedDigest string) (containerd.Image, error) {
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

	if expectedDigest != "" {
		if pinned := sourcePinnedDigest(source); pinned != "" && pinned != expectedDigest {
			return nil, fmt.Errorf("conflicting digest pins for %s: ref pins %s but --expected-digest is %s",
				source, pinned, expectedDigest)
		}
	}

	// Pulls of one reference share one trust policy drop-in, and the daemon judges
	// every pull of that reference against it. Serialize pulls so an unpinned pull
	// is not judged by a concurrent pinned pull's pin, and so the cleanup below or
	// the deferred removal never deletes another invocation's in-flight pin.
	unlock, err := lockDropin(ctx, source)
	switch {
	case err == nil:
		defer unlock()
		// With the lock held, the only drop-in that can exist here is one left
		// behind by a killed host-ctr; remove it so this pull is not judged by it.
		removeHostDigestionDropin(ctx, source)
	case (errors.Is(err, fs.ErrPermission) || errors.Is(err, fs.ErrNotExist)) && expectedDigest == "":
		// A caller that cannot create the lock cannot touch drop-ins either, so
		// it cannot race a pinned pull; let its unpinned pull proceed without one.
		log.G(ctx).WithError(err).WithField("ref", source).Debug("cannot lock digestion trust policy; pulling without it")
	default:
		return nil, fmt.Errorf("locking digestion trust policy for %s: %w", source, err)
	}

	if expectedDigest != "" {
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
		img, err := pullViaTransferService(ctx, source, client, registryConfig, labels)
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

// verifyExpectedDigest fails closed unless the digest img resolved to matches
// --expected-digest. No-op when no digest is pinned.
func verifyExpectedDigest(ctx context.Context, img containerd.Image, expectedDigest string) error {
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
func verifySourcePinnedDigest(source string, img containerd.Image) error {
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
	return fmt.Errorf("image %s resolved to digest %s, which does not match --expected-digest %s", name, got, want)
}

// isVerifierRejection reports whether err is an image verifier judgement
// rejecting the pull ("blocked pull"), which retrying would not change. A
// verifier that fails to run is reported as "blocking pull" and is retried.
func isVerifierRejection(err error) bool {
	return strings.Contains(err.Error(), "blocked pull")
}

// sourcePinnedDigest returns the digest a ref is pinned to (repo@sha256:...), or
// "" if the ref carries no digest.
func sourcePinnedDigest(source string) string {
	if i := strings.LastIndex(source, "@"); i >= 0 {
		return source[i+1:]
	}
	return ""
}

// validateExpectedDigest rejects anything but a well-formed sha256 digest.
func validateExpectedDigest(d string) error {
	parsed, err := digest.Parse(d)
	if err != nil || parsed.Algorithm() != digest.SHA256 {
		return fmt.Errorf("--expected-digest must be a sha256:<64 hex chars> digest, got %q", d)
	}
	return nil
}

// pullViaTransferService performs the daemon-side transfer-service pull: an
// OCIRegistry source (with host-ctr credentials supplied via a CredentialHelper)
// into the daemon's image store, unpacked for the default snapshotter.
func pullViaTransferService(ctx context.Context, source string, client *containerd.Client, registryConfig *RegistryConfig, labels map[string]string) (containerd.Image, error) {
	plat := platforms.DefaultSpec()

	credHelper := &hostCtrCredentialHelper{registryConfig: registryConfig}
	reg, err := tregistry.NewOCIRegistry(ctx, source,
		tregistry.WithCredentials(credHelper),
	)
	if err != nil {
		return nil, err
	}

	// Destination: the daemon's image store, unpacked for the default snapshotter.
	storeOpts := []timage.StoreOpt{
		timage.WithPlatforms(plat),
		timage.WithUnpack(plat, defaults.DefaultSnapshotter),
	}
	if len(labels) != 0 {
		storeOpts = append(storeOpts, timage.WithImageLabels(labels))
	}
	is := timage.NewStore(source, storeOpts...)

	if err := client.Transfer(ctx, reg, is, transfer.WithProgress(func(transfer.Progress) {})); err != nil {
		if credErr := credHelper.takeErr(); credErr != nil {
			return nil, fmt.Errorf("%w (credential resolution failed: %v)", err, credErr)
		}
		return nil, err
	}
	log.G(ctx).WithField("ref", source).Info("pulled via transfer service")

	return client.GetImage(ctx, source)
}

// hostCtrCredentialHelper adapts host-ctr's ECR/registry-config credential logic
// to the transfer service's registry.CredentialHelper interface.
type hostCtrCredentialHelper struct {
	registryConfig *RegistryConfig

	mu      sync.Mutex
	credErr error
}

// recordErr stashes a credential-resolution error for the caller to surface.
func (h *hostCtrCredentialHelper) recordErr(err error) {
	h.mu.Lock()
	h.credErr = err
	h.mu.Unlock()
}

// takeErr returns and clears any stashed credential error.
func (h *hostCtrCredentialHelper) takeErr() error {
	h.mu.Lock()
	defer h.mu.Unlock()
	err := h.credErr
	h.credErr = nil
	return err
}

// GetCredentials resolves credentials for host: ECR private, ECR public (with
// anonymous fallback), then the registry config's static credentials.
//
// It must never return an error: the transfer client drops a GetCredentials
// error without replying, hanging the daemon's credential Recv. On failure it
// stashes the error (for the caller to surface) and falls back to anonymous.
func (h *hostCtrCredentialHelper) GetCredentials(ctx context.Context, _ string, host string) (tregistry.Credentials, error) {
	anonymous := tregistry.Credentials{Host: host}

	// ECR private registries require authentication.
	if ecrPrivateHostPattern.MatchString(host) {
		parsed, err := parseECRHost(host)
		if err != nil {
			h.recordErr(fmt.Errorf("failed to parse ECR host %q: %w", host, err))
			log.G(ctx).WithError(err).WithField("host", host).Warn("ECR host parse failed; falling back to anonymous pull")
			return anonymous, nil
		}
		username, password, err := getECRPrivateCredentials(ctx, parsed.Region, parsed.Fips)
		if err != nil {
			h.recordErr(fmt.Errorf("failed to get private ECR credentials for region %s: %w", parsed.Region, err))
			log.G(ctx).WithError(err).WithField("host", host).Warn("private ECR credential fetch failed; falling back to anonymous pull")
			return anonymous, nil
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
				h.recordErr(fmt.Errorf("failed to parse ecr-public registry credentials: %w", err))
				log.G(ctx).WithError(err).Warn("ecr-public credential parse failed; falling back to anonymous pull")
				return anonymous, nil
			}
			return tregistry.Credentials{Host: host, Username: username, Secret: secret}, nil
		}
		username, password, err := getECRPublicCredentials(ctx)
		if err != nil {
			log.G(ctx).WithError(err).Warn("ecr-public: failed to get credentials, falling back to anonymous pull")
			return anonymous, nil
		}
		return tregistry.Credentials{Host: host, Username: username, Secret: password}, nil
	}

	// Static credentials from the registry config, if any.
	if cred, ok := h.lookupConfigCredential(host); ok {
		username, secret, err := parseAuth(cred)
		if err != nil {
			h.recordErr(fmt.Errorf("failed to parse registry credentials for %q: %w", host, err))
			log.G(ctx).WithError(err).WithField("host", host).Warn("registry credential parse failed; falling back to anonymous pull")
			return anonymous, nil
		}
		return tregistry.Credentials{Host: host, Username: username, Secret: secret}, nil
	}

	// No credentials: anonymous pull.
	return anonymous, nil
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

// dropinKey is the hex sha256 of an image reference: fixed length, so the
// drop-in and lock names fit NAME_MAX for any reference, and recomputable by
// the verifier from containerd's `-name`.
func dropinKey(reference string) string {
	sum := sha256.Sum256([]byte(reference))
	return hex.EncodeToString(sum[:])
}

// dropinPath maps an image reference to its drop-in trust policy path.
func dropinPath(reference string) string {
	return filepath.Join(hostDigestionTrustPolicyDir, dropinKey(reference)+".json")
}

// lockPath maps an image reference to its lock file, beside the drop-in.
func lockPath(reference string) string {
	return filepath.Join(hostDigestionTrustPolicyDir, dropinKey(reference)+".lock")
}

// lockDropin takes an exclusive per-reference lock for the drop-in's whole
// lifetime (stale cleanup, write, pull, removal).
func lockDropin(ctx context.Context, reference string) (func(), error) {
	return lockFile(ctx, lockPath(reference), reference)
}

// lockFile flocks path exclusively, creating it on demand.
func lockFile(ctx context.Context, path, reference string) (func(), error) {
	f, err := os.OpenFile(path, os.O_RDONLY|os.O_CREATE, 0o600)
	if err != nil {
		return nil, err
	}
	waiting := false
	for {
		err = syscall.Flock(int(f.Fd()), syscall.LOCK_EX|syscall.LOCK_NB)
		if err == nil {
			// Closing the file releases the lock.
			return func() { _ = f.Close() }, nil
		}
		if !errors.Is(err, syscall.EWOULDBLOCK) {
			_ = f.Close()
			return nil, err
		}
		if !waiting {
			log.G(ctx).WithField("ref", reference).Info("another host-ctr invocation is pulling this image; waiting for it")
			waiting = true
		}
		select {
		case <-ctx.Done():
			_ = f.Close()
			return nil, ctx.Err()
		case <-time.After(200 * time.Millisecond):
		}
	}
}

// writeHostDigestionDropin writes this image's digestion trust policy atomically.
func writeHostDigestionDropin(source, trusted string) error {
	// trusted is validated once in fetchImage before any path is taken.
	data := []byte(fmt.Sprintf(`{"version": "1.0", "trustedDigests": [%q]}`+"\n", trusted))
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
