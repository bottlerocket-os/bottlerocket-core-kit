package main

import (
	"context"
	"testing"

	"github.com/containerd/containerd/remotes/docker"
	"github.com/stretchr/testify/assert"
)

// Test RegistryHosts with valid endpoints URLs
func TestRegistryHosts(t *testing.T) {
	tests := []struct {
		name     string
		host     string
		config   RegistryConfig
		expected []docker.RegistryHost
	}{
		{
			"HTTP scheme",
			"docker.io",
			RegistryConfig{
				Mirrors: map[string]Mirror{
					"docker.io": {
						Endpoints: []string{"http://198.158.0.0"},
					},
				},
			},
			[]docker.RegistryHost{
				{
					Authorizer:   docker.NewDockerAuthorizer(),
					Host:         "198.158.0.0",
					Scheme:       "http",
					Path:         "/v2",
					Capabilities: docker.HostCapabilityResolve | docker.HostCapabilityPull,
				},
				{
					Authorizer:   docker.NewDockerAuthorizer(),
					Host:         "registry-1.docker.io",
					Scheme:       "https",
					Path:         "/v2",
					Capabilities: docker.HostCapabilityResolve | docker.HostCapabilityPull,
				},
			},
		},
		{
			"No scheme",
			"docker.io",
			RegistryConfig{
				Mirrors: map[string]Mirror{
					"docker.io": {
						Endpoints: []string{"localhost", "198.158.0.0", "127.0.0.1"},
					},
				},
			},
			[]docker.RegistryHost{
				{
					Authorizer:   docker.NewDockerAuthorizer(),
					Host:         "localhost",
					Scheme:       "http",
					Path:         "/v2",
					Capabilities: docker.HostCapabilityResolve | docker.HostCapabilityPull,
				},
				{
					Authorizer:   docker.NewDockerAuthorizer(),
					Host:         "198.158.0.0",
					Scheme:       "https",
					Path:         "/v2",
					Capabilities: docker.HostCapabilityResolve | docker.HostCapabilityPull,
				},
				{
					Authorizer:   docker.NewDockerAuthorizer(),
					Host:         "127.0.0.1",
					Scheme:       "http",
					Path:         "/v2",
					Capabilities: docker.HostCapabilityResolve | docker.HostCapabilityPull,
				},
				{
					Authorizer:   docker.NewDockerAuthorizer(),
					Host:         "registry-1.docker.io",
					Scheme:       "https",
					Path:         "/v2",
					Capabilities: docker.HostCapabilityResolve | docker.HostCapabilityPull,
				},
			},
		},
		{
			"* endpoints",
			"weird.io",
			RegistryConfig{
				Mirrors: map[string]Mirror{
					"docker.io": {
						Endpoints: []string{"notme", "certainly-not-me"},
					},
					"*": {
						Endpoints: []string{"198.158.0.0", "example.com"},
					},
				},
			},
			[]docker.RegistryHost{
				{
					Authorizer:   docker.NewDockerAuthorizer(),
					Host:         "198.158.0.0",
					Scheme:       "https",
					Path:         "/v2",
					Capabilities: docker.HostCapabilityResolve | docker.HostCapabilityPull,
				},
				{
					Authorizer:   docker.NewDockerAuthorizer(),
					Host:         "example.com",
					Scheme:       "https",
					Path:         "/v2",
					Capabilities: docker.HostCapabilityResolve | docker.HostCapabilityPull,
				},
				{
					Authorizer:   docker.NewDockerAuthorizer(),
					Host:         "weird.io",
					Scheme:       "https",
					Path:         "/v2",
					Capabilities: docker.HostCapabilityResolve | docker.HostCapabilityPull,
				},
			},
		},
		{
			"No mirrors",
			"docker.io",
			RegistryConfig{
				Mirrors: map[string]Mirror{},
			},
			[]docker.RegistryHost{
				{
					Authorizer:   docker.NewDockerAuthorizer(),
					Host:         "registry-1.docker.io",
					Scheme:       "https",
					Path:         "/v2",
					Capabilities: docker.HostCapabilityResolve | docker.HostCapabilityPull,
				},
			},
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			f := registryHosts(&tc.config, nil)
			result, err := f(tc.host)
			assert.NoError(t, err)
			assert.Equal(t, tc.expected, result)
		})
	}
}

// Test RegistryHosts with an invalid endpoint URL
func TestBadRegistryHosts(t *testing.T) {
	f := registryHosts(&RegistryConfig{
		Mirrors: map[string]Mirror{
			"docker.io": {
				Endpoints: []string{"$#%#$$#%#$"},
			},
		},
	}, nil)
	_, err := f("docker.io")
	assert.Error(t, err)
}

func TestParseImageURIAsECR(t *testing.T) {
	tests := []struct {
		name           string
		ecrImgURI      string
		expectedErr    bool
		expectedResult *parsedECR
	}{
		{
			"Parse typical region for normal use-cases",
			"777777777777.dkr.ecr.us-west-2.amazonaws.com/my_image:latest",
			false,
			&parsedECR{
				Account:  "777777777777",
				Region:   "us-west-2",
				RepoPath: "my_image:latest",
				Fips:     false,
			},
		},
		{
			"Parse China regions",
			"777777777777.dkr.ecr.cn-north-1.amazonaws.com.cn/my_image:latest",
			false,
			&parsedECR{
				Account:  "777777777777",
				Region:   "cn-north-1",
				RepoPath: "my_image:latest",
				Fips:     false,
			},
		},
		{
			"Parse special region",
			"777777777777.dkr.ecr.eu-isoe-west-1.cloud.adc-e.uk/my_image:latest",
			false,
			&parsedECR{
				Account:  "777777777777",
				Region:   "eu-isoe-west-1",
				RepoPath: "my_image:latest",
				Fips:     false,
			},
		},
		{
			"Parse FIPS region for normal use-cases",
			"777777777777.dkr.ecr-fips.us-west-2.amazonaws.com/my_image:latest",
			false,
			&parsedECR{
				Account:  "777777777777",
				Region:   "us-west-2",
				RepoPath: "my_image:latest",
				Fips:     true,
			},
		},
		{
			"Fail for no region",
			"111111111111.dkr.ecr..amazonaws.com/bottlerocket/container:1.2.3",
			true,
			nil,
		},
		{
			"Empty string fails",
			"",
			true,
			nil,
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			result, err := parseImageURIAsECR(tc.ecrImgURI)
			if tc.expectedErr {
				// handle error cases
				if err == nil {
					t.Fail()
				}
			} else {
				// handle happy paths
				assert.Equal(t, tc.expectedResult, result)
			}
		})
	}
}

func TestFetchECRRef(t *testing.T) {
	specialRegions := specialRegions{
		FipsSupportedEcrRegions: map[string]bool{
			"us-east-1":     true,
			"us-east-2":     true,
			"us-west-1":     true,
			"us-west-2":     true,
			"us-gov-east-1": true,
			"us-gov-west-1": true,
		},
		EcrRefPrefixMappings: map[string]string{
			"ap-southeast-7": "ecr.aws/arn:aws:ecr:ap-southeast-7:",
			"eu-isoe-west-1": "ecr.aws/arn:aws-iso-e:ecr:eu-isoe-west-1:",
			"mx-central-1":   "ecr.aws/arn:aws:ecr:mx-central-1:",
		},
	}
	tests := []struct {
		name        string
		ecrImgURI   string
		expectedErr bool
		expectedRef string
	}{
		{
			"Parse typical region for normal use-cases",
			"111111111111.dkr.ecr.us-west-2.amazonaws.com/bottlerocket/container:1.2.3",
			false,
			"ecr.aws/arn:aws:ecr:us-west-2:111111111111:repository/bottlerocket/container:1.2.3",
		},
		{
			"Parse special region",
			"111111111111.dkr.ecr.eu-isoe-west-1.amazonaws.com/bottlerocket/container:1.2.3",
			false,
			"ecr.aws/arn:aws-iso-e:ecr:eu-isoe-west-1:111111111111:repository/bottlerocket/container:1.2.3",
		},
		{
			"Parse special region",
			"111111111111.dkr.ecr.mx-central-1.amazonaws.com/bottlerocket-control:v0.7.17",
			false,
			"ecr.aws/arn:aws:ecr:mx-central-1:111111111111:repository/bottlerocket-control:v0.7.17",
		},
		{
			"Parse China regions",
			"111111111111.dkr.ecr.cn-north-1.amazonaws.com/bottlerocket/container:1.2.3",
			false,
			"ecr.aws/arn:aws-cn:ecr:cn-north-1:111111111111:repository/bottlerocket/container:1.2.3",
		},
		{
			"Parse gov regions",
			"111111111111.dkr.ecr.us-gov-west-1.amazonaws.com/bottlerocket/container:1.2.3",
			false,
			"ecr.aws/arn:aws-us-gov:ecr:us-gov-west-1:111111111111:repository/bottlerocket/container:1.2.3",
		},
		{
			"Parse FIPS region for normal use-cases",
			"111111111111.dkr.ecr-fips.us-west-2.amazonaws.com/bottlerocket/container:1.2.3",
			false,
			"ecr.aws/arn:aws:ecr-fips:us-west-2:111111111111:repository/bottlerocket/container:1.2.3",
		},
		{
			"Fail for region that does not have FIPS support",
			"111111111111.dkr.ecr-fips.ca-central-1.amazonaws.com/bottlerocket/container:1.2.3",
			true,
			"",
		},
		{
			"Fail for invalid region",
			"111111111111.dkr.ecr.outer-space.amazonaws.com/bottlerocket/container:1.2.3",
			true,
			"",
		},
		{
			"Empty string fails",
			"",
			true,
			"",
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			result, err := fetchECRRef(context.TODO(), tc.ecrImgURI, specialRegions)
			if tc.expectedErr {
				// handle error cases
				if err == nil {
					t.Fail()
				}
			} else {
				// handle happy paths
				assert.Equal(t, tc.expectedRef, result.Canonical())
			}
		})
	}
}

func TestConvertLabel(t *testing.T) {
	tests := []struct {
		name             string
		labels           []string
		expectedErr      bool
		expectedLabelMap map[string]string
	}{
		{
			"Valid single label",
			[]string{"io.cri-containerd.pinned=pinned"},
			false,
			map[string]string{
				"io.cri-containerd.pinned": "pinned",
			},
		},
		{
			"Valid single label without equals sign",
			[]string{"io.cri-containerd.pinned,pinned"},
			false,
			map[string]string{
				"io.cri-containerd.pinned,pinned": "",
			},
		},
		{
			"Empty labels",
			[]string{""},
			false,
			map[string]string{"": ""},
		},
		{
			"Valid multiple labels",
			[]string{"io.cri-containerd.pinned=pinned", "io.cri-containerd.test=test"},
			false,
			map[string]string{
				"io.cri-containerd.pinned": "pinned",
				"io.cri-containerd.test":   "test",
			},
		},
		{
			"valid multiple labels without equals sign",
			[]string{"io.cri-containerd.pinned=pinned", "io.cri-containerd.test,test"},
			false,
			map[string]string{
				"io.cri-containerd.pinned":    "pinned",
				"io.cri-containerd.test,test": "",
			},
		},
		{
			"Value is empty",
			[]string{"io.cri-containerd.pinned=pinned", "io.cri-containerd.test="},
			false,
			map[string]string{
				"io.cri-containerd.pinned": "pinned",
				"io.cri-containerd.test":   "",
			},
		},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			result, err := convertLabels(tc.labels)
			if tc.expectedErr {
				// handle error cases
				if err == nil {
					t.Fail()
				}
			} else {
				// handle happy paths
				assert.Equal(t, tc.expectedLabelMap, result)
			}
		})
	}
}
