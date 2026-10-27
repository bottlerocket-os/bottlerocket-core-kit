package main

import "testing"

func TestSourcePinnedDigest(t *testing.T) {
	cases := map[string]string{
		"example.com/repo:tag":                                     "",
		"example.com/repo":                                         "",
		"example.com/repo@sha256:" + hexN(64):                      "sha256:" + hexN(64),
		"123456789012.dkr.ecr.us-west-2.amazonaws.com/x@sha256:ab": "sha256:ab",
	}
	for in, want := range cases {
		if got := sourcePinnedDigest(in); got != want {
			t.Errorf("sourcePinnedDigest(%q) = %q, want %q", in, got, want)
		}
	}
}

func TestIsValidSha256Digest(t *testing.T) {
	valid := []string{"sha256:" + hexN(64)}
	invalid := []string{
		"",
		"sha256:",                  // empty hex (finding #7)
		"sha256:" + hexN(63),       // too short
		"sha256:" + hexN(65),       // too long
		"sha256:" + "g" + hexN(63), // non-hex
		"sha512:" + hexN(64),       // wrong algo
		hexN(64),                   // missing prefix
	}
	for _, d := range valid {
		if !isValidSha256Digest(d) {
			t.Errorf("isValidSha256Digest(%q) = false, want true", d)
		}
	}
	for _, d := range invalid {
		if isValidSha256Digest(d) {
			t.Errorf("isValidSha256Digest(%q) = true, want false", d)
		}
	}
}

func TestDigestMismatch(t *testing.T) {
	d := "sha256:" + hexN(64)
	// Matching digest passes.
	if err := digestMismatch("repo:tag", d, d); err != nil {
		t.Errorf("digestMismatch(match) = %v, want nil", err)
	}
	// Any difference fails closed.
	if err := digestMismatch("repo:tag", d, "sha256:"+hexN(63)+"b"); err == nil {
		t.Error("digestMismatch(mismatch) = nil, want error")
	}
	// A pinned digest never matches an empty resolved digest.
	if err := digestMismatch("repo:tag", "", d); err == nil {
		t.Error("digestMismatch(empty got) = nil, want error")
	}
}

func TestSourcePinMismatch(t *testing.T) {
	d := "sha256:" + hexN(64)
	// Unpinned source: any resolved digest is accepted (no-op).
	if err := sourcePinMismatch("example.com/repo:tag", d); err != nil {
		t.Errorf("sourcePinMismatch(unpinned) = %v, want nil", err)
	}
	// Pinned source that resolves to its own digest passes.
	if err := sourcePinMismatch("example.com/repo@"+d, d); err != nil {
		t.Errorf("sourcePinMismatch(match) = %v, want nil", err)
	}
	// Pinned source that resolves to a different manifest fails closed.
	evil := "sha256:" + hexN(63) + "b"
	if err := sourcePinMismatch("example.com/repo@"+d, evil); err == nil {
		t.Error("sourcePinMismatch(mismatch) = nil, want error")
	}
}

func hexN(n int) string {
	b := make([]byte, n)
	for i := range b {
		b[i] = 'a'
	}
	return string(b)
}
