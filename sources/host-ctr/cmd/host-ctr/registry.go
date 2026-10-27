package main

import (
	"encoding/base64"
	"fmt"
	"net"
	"net/http"
	"net/url"
	"os"
	"strings"
	"time"

	"github.com/containerd/containerd/v2/core/remotes/docker"
	"github.com/pelletier/go-toml"
	"github.com/pkg/errors"
)

// Mirror contains the config related to the registry mirror
type Mirror struct {
	Endpoints []string `toml:"endpoints,omitempty"`
}

// Credential contains a registry credential
type Credential struct {
	Username      string `toml:"username,omitempty"`
	Password      string `toml:"password,omitempty"`
	Auth          string `toml:"auth,omitempty"`
	IdentityToken string `toml:"identitytoken,omitempty"`
}

// RegistryConfig contains the config related to a image registry
type RegistryConfig struct {
	Mirrors     map[string]Mirror     `toml:"mirrors,omitempty"`
	Credentials map[string]Credential `toml:"creds,omitempty"`
}

// NewRegistryConfig unmarshalls a registry configuration file and sets up a RegistryConfig
func NewRegistryConfig(registryConfigFile string) (*RegistryConfig, error) {
	raw, err := os.ReadFile(registryConfigFile)
	if err != nil {
		return nil, err
	}

	config := RegistryConfig{}
	return &config, toml.Unmarshal(raw, &config)
}

// NewBlankRegistryConfig sets up a RegistryConfig without unmarshalling any toml
func NewBlankRegistryConfig() *RegistryConfig {
	config := RegistryConfig{}
	// no config to unmarshal
	return &config
}

// registryHosts returns the registry hosts to be used by the resolver.
// Heavily borrowed from containerd CRI plugin's implementation.
// See https://github.com/containerd/containerd/blob/1407cab509ff0d96baa4f0eb6ff9980270e6e620/pkg/cri/server/image_pull.go#L332-L405
// authorizerOverride lets the caller override the generated authorizer with a custom authorizer
// FIXME Replace this once there's a public containerd client interface that supports registry mirrors
func registryHosts(registryConfig *RegistryConfig, authorizerOverride *docker.Authorizer) docker.RegistryHosts {
	return func(host string) ([]docker.RegistryHost, error) {
		var (
			registries []docker.RegistryHost
			endpoints  []string
		)
		// Set up endpoints for the registry
		// The endpoints array may still be empty after this step
		if registryConfig != nil {
			if _, ok := registryConfig.Mirrors[host]; ok {
				endpoints = registryConfig.Mirrors[host].Endpoints
			} else if _, ok := registryConfig.Mirrors["*"]; ok {
				endpoints = registryConfig.Mirrors["*"].Endpoints
			}
		}
		defaultHost, err := docker.DefaultHost(host)
		if err != nil {
			return nil, errors.Wrap(err, "get default host")
		}
		// append this default host to the possibly-empty endpoints array
		endpoints = append(endpoints, defaultHost)

		for _, endpoint := range endpoints {
			// Prefix the endpoint with an appropriate URL scheme if the endpoint does not have one.
			if !strings.Contains(endpoint, "://") {
				if endpoint == "localhost" || endpoint == "127.0.0.1" || endpoint == "::1" {
					endpoint = "http://" + endpoint
				} else {
					endpoint = "https://" + endpoint
				}
			}
			url, err := url.Parse(endpoint)
			if err != nil {
				return nil, errors.Wrapf(err, "parse registry endpoint %q from mirrors", endpoint)
			}
			if url.Path == "" {
				url.Path = "/v2"
			}
			var authorizer docker.Authorizer
			if authorizerOverride == nil {
				// Set up auth for pulling from registry
				var authOpts []docker.AuthorizerOpt
				if cred, ok := registryConfig.Credentials[defaultHost]; ok {
					authOpts = append(authOpts, docker.WithAuthClient(&http.Client{
						Transport: newTransport(),
					}))
					authOpts = append(authOpts, docker.WithAuthCreds(func(_ string) (string, string, error) {
						return parseAuth(cred)
					}))
				}
				authorizer = docker.NewDockerAuthorizer(authOpts...)
			} else {
				authorizer = *authorizerOverride
			}
			registries = append(registries, docker.RegistryHost{
				Authorizer:   authorizer,
				Host:         url.Host,
				Scheme:       url.Scheme,
				Path:         url.Path,
				Capabilities: docker.HostCapabilityResolve | docker.HostCapabilityPull,
			})
		}
		return registries, nil
	}
}

// parseAuth resolves a registry Credential into a (username, secret) pair: prefer
// explicit username/password, then an identity token, then a base64-encoded
// `user:password` `auth` blob. Reimplements containerd's CRI ParseAuth, which
// moved under an internal/ path in v2 and is no longer importable:
// https://github.com/containerd/containerd/blob/2892c2042ee7fbd3be0e5bdc675e07b5acedb0bf/pkg/cri/server/image_pull.go#L235
func parseAuth(cred Credential) (string, string, error) {
	if cred.Username != "" {
		return cred.Username, cred.Password, nil
	}
	if cred.IdentityToken != "" {
		return "", cred.IdentityToken, nil
	}
	if cred.Auth != "" {
		decoded, err := base64.StdEncoding.DecodeString(cred.Auth)
		if err != nil {
			return "", "", err
		}
		fields := strings.SplitN(string(decoded), ":", 2)
		if len(fields) != 2 {
			return "", "", fmt.Errorf("invalid decoded auth: %q", string(decoded))
		}
		user, passwd := fields[0], strings.Trim(fields[1], "\x00")
		return user, passwd, nil
	}
	return "", "", nil
}

// newTransport is borrowed from containerd CRI plugin
// See https://github.com/containerd/containerd/blob/1407cab509ff0d96baa4f0eb6ff9980270e6e620/pkg/cri/server/image_pull.go#L466-L481
// FIXME Replace this once containerd creates a library that shares this code with ctr
func newTransport() *http.Transport {
	return &http.Transport{
		Proxy: http.ProxyFromEnvironment,
		DialContext: (&net.Dialer{
			Timeout:       30 * time.Second,
			KeepAlive:     30 * time.Second,
			FallbackDelay: 300 * time.Millisecond,
		}).DialContext,
		MaxIdleConns:          10,
		IdleConnTimeout:       30 * time.Second,
		TLSHandshakeTimeout:   10 * time.Second,
		ExpectContinueTimeout: 5 * time.Second,
	}
}
