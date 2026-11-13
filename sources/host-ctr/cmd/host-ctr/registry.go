package main

import (
	"encoding/base64"
	"fmt"
	"os"
	"strings"

	"github.com/pelletier/go-toml"
)

// Credential contains a registry credential
type Credential struct {
	Username      string `toml:"username,omitempty"`
	Password      string `toml:"password,omitempty"`
	Auth          string `toml:"auth,omitempty"`
	IdentityToken string `toml:"identitytoken,omitempty"`
}

// RegistryConfig contains the config related to a image registry
type RegistryConfig struct {
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

/*
 *  Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
 *  Copyright The containerd Authors.
 *
 *  SPDX-License-Identifier: Apache-2.0
 *  Originally derived from:
 *  https://github.com/containerd/containerd/blob/2892c2042ee7fbd3be0e5bdc675e07b5acedb0bf/pkg/cri/server/image_pull.go#L235
 *
 *  Changes for Bottlerocket:
 *    - Take a registry-config Credential instead of a CRI AuthConfig and host
 *    - Drop the ServerAddress host check; credentials are already keyed by host
 *    - Do not include the decoded value in the malformed-auth error
 */

// parseAuth resolves a registry Credential into a (username, secret) pair: prefer
// explicit username/password, then an identity token, then a base64-encoded
// `user:password` `auth` blob.
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
			return "", "", fmt.Errorf("invalid decoded auth: expected user:password")
		}
		user, passwd := fields[0], strings.Trim(fields[1], "\x00")
		return user, passwd, nil
	}
	return "", "", nil
}
