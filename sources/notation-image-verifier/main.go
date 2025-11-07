// package main provides a notation CLI wrapper that implements the containerd
// image verification plugin interface as defined at:
// https://github.com/containerd/containerd/blob/main/docs/image-verification.md
//
// The verifier receives:
// - `-name`: The given reference to the image that may be pulled
// - `-digest`: The resolved digest of the image that may be pulled
// - `-stdin-media-type`: The media type of JSON data passed to stdin
//
// Returns exit code 0 to allow image pull, non-zero to block.
package main

import (
	"flag"
	"fmt"
	"os"
	"os/exec"
	"strings"
)

// constructImageReference creates a notation-compatible image reference
// by combining the URI (name without tag) with the digest.
// Example: "docker.io/library/hello-world:latest" + "sha256:abc123"
//
//	-> "docker.io/library/hello-world@sha256:abc123"
func constructImageReference(name, digest string) string {
	// If name already contains a digest, remove it first
	if atIndex := strings.Index(name, "@"); atIndex != -1 {
		name = name[:atIndex]
	}

	// Remove tag if present (everything after the last colon that's not part of a port)
	uri := name
	if lastColon := strings.LastIndex(name, ":"); lastColon != -1 {
		// Check if this colon is part of a hostname:port (contains slash after colon)
		if !strings.Contains(name[lastColon:], "/") {
			uri = name[:lastColon]
		}
	}
	return uri + "@" + digest
}

func main() {
	var digest = flag.String("digest", "", "image digest to verify")
	var name = flag.String("name", "", "image name to verify")
	var _ = flag.String("stdin-media-type", "", "image media type")
	flag.Parse()

	if *digest == "" || *name == "" {
		// containerd only captures stdout, so no stderr output per interface spec
		fmt.Printf("Usage: %s -digest <digest> -name <name>\n", os.Args[0])
		os.Exit(1)
	}

	imageRef := constructImageReference(*name, *digest)
	fmt.Printf("verifying image: %s\n", imageRef)

	// Override the default notation paths to what's packaged in packages/notation
	// https://notaryproject.dev/docs/user-guides/how-to/directory-structure/
	os.Setenv("NOTATION_CONFIG", "/etc/notation")
	os.Setenv("NOTATION_CACHE", "/var/cache/notation")
	os.Setenv("NOTATION_LIBEXEC", "/usr/libexec/notation-plugins")

	// Bottlerocket does not have a $HOME set by default and notation expect to find
	// credentials from the ecr-credential-helper here.
	os.Setenv("HOME", "/root")

	cmd := exec.Command("notation", "verify", imageRef)
	output, err := cmd.CombinedOutput()

	if err != nil {
		fmt.Printf("image verification failed: %s\n", string(output))
		os.Exit(1)
	}

	fmt.Println("image verification successful")
	os.Exit(0)
}
