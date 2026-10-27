package main

import (
	"context"
	"testing"

	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/containerd/containerd/v2/core/remotes/docker"
	"github.com/stretchr/testify/assert"
)

// Test RegistryHosts with valid endpoints URLs
func TestRegistryHosts(t *testing.T) {
	tests := []struct {
		name   string
		host   string
		config RegistryConfig
		want   []docker.RegistryHost
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
			assert.Equal(t, tc.want, result)
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

func TestParseECRHost(t *testing.T) {
	tests := []struct {
		name       string
		host       string
		wantErr    bool
		wantResult *parsedECR
	}{
		{"us-west-2", "777777777777.dkr.ecr.us-west-2.amazonaws.com", false, &parsedECR{Region: "us-west-2", Fips: false}},
		{"cn-north-1", "777777777777.dkr.ecr.cn-north-1.amazonaws.com.cn", false, &parsedECR{Region: "cn-north-1", Fips: false}},
		{"eu-isoe-west-1", "777777777777.dkr.ecr.eu-isoe-west-1.cloud.adc-e.uk", false, &parsedECR{Region: "eu-isoe-west-1", Fips: false}},
		{"eusc-de-east-1", "777777777777.dkr.ecr.eusc-de-east-1.amazonaws.eu", false, &parsedECR{Region: "eusc-de-east-1", Fips: false}},
		{"us-iso-east-1", "777777777777.dkr.ecr.us-iso-east-1.c2s.ic.gov", false, &parsedECR{Region: "us-iso-east-1", Fips: false}},
		{"us-isob-east-1", "777777777777.dkr.ecr.us-isob-east-1.sc2s.sgov.gov", false, &parsedECR{Region: "us-isob-east-1", Fips: false}},
		{"us-isof-east-1", "777777777777.dkr.ecr.us-isof-east-1.csp.hci.ic.gov", false, &parsedECR{Region: "us-isof-east-1", Fips: false}},
		{"fips us-west-2", "777777777777.dkr.ecr-fips.us-west-2.amazonaws.com", false, &parsedECR{Region: "us-west-2", Fips: true}},
		{"invalid FIPS region", "111111111111.dkr.ecr-fips.eu-west-1.amazonaws.com", true, nil},
		{"empty string", "", true, nil},
		{"non-ECR domain", "111111111111.dkr.ecr.us-west-2.amazonaws.com.example.org", true, nil},
		{"unrecognized domain suffix", "111111111111.dkr.ecr.us-west-2.notamazon.com", true, nil},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			result, err := parseECRHost(tc.host)
			if tc.wantErr {
				assert.Error(t, err)
				return
			}
			assert.NoError(t, err)
			assert.Equal(t, tc.wantResult, result)
		})
	}
}

func TestECRRegionHandling(t *testing.T) {
	tests := []struct {
		name     string
		host     string
		wantFips bool
		wantErr  bool
	}{
		{"us-west-2", "111111111111.dkr.ecr.us-west-2.amazonaws.com", false, false},
		{"cn-north-1", "111111111111.dkr.ecr.cn-north-1.amazonaws.com.cn", false, false},
		{"eu-isoe-west-1", "111111111111.dkr.ecr.eu-isoe-west-1.cloud.adc-e.uk", false, false},
		{"eusc-de-east-1", "111111111111.dkr.ecr.eusc-de-east-1.amazonaws.eu", false, false},
		{"us-iso-east-1", "111111111111.dkr.ecr.us-iso-east-1.c2s.ic.gov", false, false},
		{"us-iso-west-1", "111111111111.dkr.ecr.us-iso-west-1.c2s.ic.gov", false, false},
		{"us-isob-east-1", "111111111111.dkr.ecr.us-isob-east-1.sc2s.sgov.gov", false, false},
		{"us-isob-west-1", "111111111111.dkr.ecr.us-isob-west-1.sc2s.sgov.gov", false, false},
		{"us-isof-east-1", "111111111111.dkr.ecr.us-isof-east-1.csp.hci.ic.gov", false, false},
		{"us-isof-south-1", "111111111111.dkr.ecr.us-isof-south-1.csp.hci.ic.gov", false, false},
		{"ap-southeast-7", "111111111111.dkr.ecr.ap-southeast-7.amazonaws.com", false, false},
		{"mx-central-1", "111111111111.dkr.ecr.mx-central-1.amazonaws.com", false, false},
		{"ap-east-2", "111111111111.dkr.ecr.ap-east-2.amazonaws.com", false, false},
		{"ap-southeast-6", "111111111111.dkr.ecr.ap-southeast-6.amazonaws.com", false, false},
		{"us-northeast-1", "111111111111.dkr.ecr.us-northeast-1.amazonaws.com", false, false},
		{"fips us-west-2", "111111111111.dkr.ecr-fips.us-west-2.amazonaws.com", true, false},
		{"fips us-gov-west-1", "111111111111.dkr.ecr-fips.us-gov-west-1.amazonaws.com", true, false},
		{"missing account ID", "dkr.ecr.us-west-2.amazonaws.com", false, true},
		{"malformed host", "not-an-ecr-host", false, true},
	}

	for _, tc := range tests {
		t.Run(tc.name, func(t *testing.T) {
			parsed, err := parseECRHost(tc.host)
			if tc.wantErr {
				assert.Error(t, err)
				return
			}
			assert.NoError(t, err)
			assert.NotEmpty(t, parsed.Region)
			assert.Equal(t, tc.wantFips, parsed.Fips)

			cfg, err := config.LoadDefaultConfig(context.Background(), config.WithRegion(parsed.Region))
			assert.NoError(t, err, "AWS SDK should support region %s", parsed.Region)
			assert.Equal(t, parsed.Region, cfg.Region)
		})
	}
}

func TestConvertLabel(t *testing.T) {
	tests := []struct {
		name         string
		labels       []string
		wantErr      bool
		wantLabelMap map[string]string
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
			if tc.wantErr {
				// handle error cases
				if err == nil {
					t.Fail()
				}
			} else {
				// handle happy paths
				assert.Equal(t, tc.wantLabelMap, result)
			}
		})
	}
}

func TestDecodeECRToken(t *testing.T) {
	tests := []struct {
		name     string
		token    *string
		wantUser string
		wantPass string
		wantErr  bool
	}{
		{
			name:     "Valid token",
			token:    stringPtr("QVdTOnBhc3N3b3Jk"), // base64("AWS:password")
			wantUser: "AWS",
			wantPass: "password",
			wantErr:  false,
		},
		{
			name:    "Nil token",
			token:   nil,
			wantErr: true,
		},
		{
			name:    "Invalid base64",
			token:   stringPtr("not-valid-base64!@#"),
			wantErr: true,
		},
		{
			name:    "Missing colon separator",
			token:   stringPtr("QVdTcGFzc3dvcmQ="), // base64("AWSpassword")
			wantErr: true,
		},
		{
			name:    "Empty token",
			token:   stringPtr(""),
			wantErr: true,
		},
		{
			name:     "Password with colons",
			token:    stringPtr("QVdTOnBhc3M6d29yZDp3aXRoOmNvbG9ucw=="), // base64("AWS:pass:word:with:colons")
			wantUser: "AWS",
			wantPass: "pass:word:with:colons",
			wantErr:  false,
		},
		{
			name:     "Password with special chars",
			token:    stringPtr("QVdTOnBAc3MhdzByZCMkJQ=="), // base64("AWS:p@ss!w0rd#$%")
			wantUser: "AWS",
			wantPass: "p@ss!w0rd#$%",
			wantErr:  false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			user, pass, err := decodeECRToken(tt.token)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
				assert.Equal(t, tt.wantUser, user)
				assert.Equal(t, tt.wantPass, pass)
			}
		})
	}
}

func TestExtractHostFromRef(t *testing.T) {
	tests := []struct {
		name      string
		ref       string
		wantHost  string
		wantError bool
	}{
		{
			name:      "ECR private registry",
			ref:       "123456789012.dkr.ecr.us-west-2.amazonaws.com/my-repo:latest",
			wantHost:  "123456789012.dkr.ecr.us-west-2.amazonaws.com",
			wantError: false,
		},
		{
			name:      "ECR public registry",
			ref:       "public.ecr.aws/my-repo:latest",
			wantHost:  "public.ecr.aws",
			wantError: false,
		},
		{
			name:      "Docker Hub",
			ref:       "docker.io/library/nginx:latest",
			wantHost:  "docker.io",
			wantError: false,
		},
		{
			name:      "Quay.io registry",
			ref:       "quay.io/prometheus/node-exporter:v1.5.0",
			wantHost:  "quay.io",
			wantError: false,
		},
		{
			name:      "Invalid reference",
			ref:       "not a valid reference",
			wantError: true,
		},
		{
			name:      "Empty reference",
			ref:       "",
			wantError: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			host, err := extractHostFromRef(tt.ref)
			if tt.wantError {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
				assert.Equal(t, tt.wantHost, host)
			}
		})
	}
}

// Helper function for tests
func stringPtr(s string) *string {
	return &s
}
