package main

import (
	"context"
	"errors"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

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

func TestValidateExpectedDigest(t *testing.T) {
	valid := []string{"sha256:" + hexN(64)}
	invalid := []string{
		"",
		"sha256:",                  // empty hex
		"sha256:" + hexN(63),       // too short
		"sha256:" + hexN(65),       // too long
		"sha256:" + "g" + hexN(63), // non-hex
		"sha512:" + hexN(64),       // wrong algo
		hexN(64),                   // missing prefix
	}
	for _, d := range valid {
		if err := validateExpectedDigest(d); err != nil {
			t.Errorf("validateExpectedDigest(%q) = %v, want nil", d, err)
		}
	}
	for _, d := range invalid {
		if err := validateExpectedDigest(d); err == nil {
			t.Errorf("validateExpectedDigest(%q) = nil, want error", d)
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

func TestLockFileExcludesConcurrentHolders(t *testing.T) {
	dir := t.TempDir()
	path := filepath.Join(dir, "repo.lock")
	ref := "example.com/repo:tag"
	unlock, err := lockFile(context.Background(), path, ref)
	if err != nil {
		t.Fatalf("first lock: %v", err)
	}
	// While held, a second holder of the same lock waits and gives up when
	// its context ends.
	ctx, cancel := context.WithTimeout(context.Background(), 300*time.Millisecond)
	defer cancel()
	if _, err := lockFile(ctx, path, ref); !errors.Is(err, context.DeadlineExceeded) {
		t.Fatalf("second lock while held: err = %v, want DeadlineExceeded", err)
	}
	// A different lock file is independent.
	unlockOther, err := lockFile(context.Background(), filepath.Join(dir, "other.lock"), "example.com/other:tag")
	if err != nil {
		t.Fatalf("lock of other file while first held: %v", err)
	}
	unlockOther()
	// Released: the same lock can be taken again.
	unlock()
	unlockAgain, err := lockFile(context.Background(), path, ref)
	if err != nil {
		t.Fatalf("relock after unlock: %v", err)
	}
	unlockAgain()
}

func TestDropinAndLockPathsShareKey(t *testing.T) {
	ref := "example.com/repo:tag"
	key := dropinKey(ref)
	if dropinPath(ref) != hostDigestionTrustPolicyDir+"/"+key+".json" {
		t.Errorf("dropinPath(%q) = %q", ref, dropinPath(ref))
	}
	if lockPath(ref) != hostDigestionTrustPolicyDir+"/"+key+".lock" {
		t.Errorf("lockPath(%q) = %q", ref, lockPath(ref))
	}
}

func TestDropinKeyIsSha256Hex(t *testing.T) {
	// Must match the digestion verifier's computation of the same reference.
	if got := dropinKey("example.com/image:tag"); got != "d3985c25ba6d24b6662bc9de2c6e5844cf376c4d3464a82d5df94de0b270d86a" {
		t.Errorf("dropinKey = %q", got)
	}
}

func TestDropinNamesFitNameMax(t *testing.T) {
	ref := strings.Repeat("r", 1024)
	for _, p := range []string{dropinPath(ref), dropinPath(ref) + ".tmp", lockPath(ref)} {
		if n := len(filepath.Base(p)); n > 255 {
			t.Errorf("%q is %d chars, exceeds NAME_MAX", filepath.Base(p), n)
		}
	}
}

func hexN(n int) string {
	b := make([]byte, n)
	for i := range b {
		b[i] = 'a'
	}
	return string(b)
}
