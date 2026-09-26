# Cached-layer snapshot regression

`TestPullImageCachedSnapshotSurvivesGC` requires an isolated Linux containerd with
its default overlayfs snapshotter. Set `HOST_CTR_TEST_SOCKET` to that disposable
daemon's Unix socket and run:

```sh
HOST_CTR_TEST_SOCKET=/work/containerd.sock go test ./cmd/host-ctr -run TestPullImageCachedSnapshotSurvivesGC -count=1 -v
```

The test seeds two synthetic image manifests sharing an empty filesystem layer,
serves the new manifest from a loopback HTTP registry, and calls the real
`pullImage`. A snapshotter wrapper synchronously deletes the old image at the
cached-layer boundary. The old separate unpack path loses the parent snapshot;
the integrated pull/unpack path retains it. Another synchronous collection after
`pullImage` returns checks that container snapshot creation remains possible.

No image is executed and no registry credentials or external network are used.
Run against a disposable daemon: synthetic namespace contents are intentionally
left available for inspection. Without the environment variable the test skips.
