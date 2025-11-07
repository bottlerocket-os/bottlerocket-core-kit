package main

import "testing"

func TestConstructImageReference(t *testing.T) {
	tests := []struct {
		name     string
		imageName string
		digest   string
		expected string
	}{
		{
			name:     "simple image with tag",
			imageName: "docker.io/library/hello-world:latest",
			digest:   "sha256:abc123",
			expected: "docker.io/library/hello-world@sha256:abc123",
		},
		{
			name:     "image without tag",
			imageName: "docker.io/library/hello-world",
			digest:   "sha256:def456",
			expected: "docker.io/library/hello-world@sha256:def456",
		},
		{
			name:     "image with port in hostname",
			imageName: "localhost:5000/myimage:v1.0",
			digest:   "sha256:ghi789",
			expected: "localhost:5000/myimage@sha256:ghi789",
		},
		{
			name:     "image with port but no tag",
			imageName: "localhost:5000/myimage",
			digest:   "sha256:jkl012",
			expected: "localhost:5000/myimage@sha256:jkl012",
		},
		{
			name:     "complex tag with version",
			imageName: "registry.example.com/namespace/image:v1.2.3-alpha",
			digest:   "sha256:mno345",
			expected: "registry.example.com/namespace/image@sha256:mno345",
		},
		{
			name:     "image with existing digest",
			imageName: "docker.io/library/hello-world@sha256:old123",
			digest:   "sha256:new456",
			expected: "docker.io/library/hello-world@sha256:new456",
		},
		{
			name:     "image with digest and tag",
			imageName: "registry.example.com/image:latest@sha256:old789",
			digest:   "sha256:new012",
			expected: "registry.example.com/image@sha256:new012",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := constructImageReference(tt.imageName, tt.digest)
			if result != tt.expected {
				t.Errorf("constructImageReference(%q, %q) = %q, want %q", 
					tt.imageName, tt.digest, result, tt.expected)
			}
		})
	}
}
