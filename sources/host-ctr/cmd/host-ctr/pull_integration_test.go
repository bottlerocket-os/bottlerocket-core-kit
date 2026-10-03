package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"sync"
	"testing"
	"time"

	"github.com/containerd/containerd"
	"github.com/containerd/containerd/content"
	"github.com/containerd/containerd/errdefs"
	"github.com/containerd/containerd/images"
	"github.com/containerd/containerd/mount"
	"github.com/containerd/containerd/namespaces"
	"github.com/containerd/containerd/snapshots"
	digest "github.com/opencontainers/go-digest"
	spec "github.com/opencontainers/image-spec/specs-go"
	oci "github.com/opencontainers/image-spec/specs-go/v1"
	"github.com/stretchr/testify/require"
)

// gcBoundary removes the old image at the cached-layer boundary of either unpacker.
// Synchronous GC makes the otherwise timing-dependent loss reproducible.
type gcBoundary struct {
	snapshots.Snapshotter
	client *containerd.Client
	parent string
	once   sync.Once
	fired  bool
	err    error
}

func (b *gcBoundary) collect(ctx context.Context) {
	b.once.Do(func() {
		b.fired = true
		b.err = b.client.ImageService().Delete(ctx, "example.invalid/test:old", images.SynchronousDelete())
	})
}

func (b *gcBoundary) Stat(ctx context.Context, key string) (snapshots.Info, error) {
	info, err := b.Snapshotter.Stat(ctx, key)
	if err == nil && key == b.parent {
		b.collect(ctx)
	}
	return info, err
}

func (b *gcBoundary) Prepare(ctx context.Context, key, parent string, opts ...snapshots.Opt) ([]mount.Mount, error) {
	mounts, err := b.Snapshotter.Prepare(ctx, key, parent, opts...)
	// Pull's parallel unpacker leases the cached snapshot through Prepare first.
	if errdefs.IsAlreadyExists(err) {
		b.collect(ctx)
	}
	return mounts, err
}

func TestPullImageCachedSnapshotSurvivesGC(t *testing.T) {
	// Opt in against a disposable Linux containerd with the default snapshotter.
	// No registry, credentials, image execution or external network is needed.
	socket := os.Getenv("HOST_CTR_TEST_SOCKET")
	if socket == "" {
		t.Skip("set HOST_CTR_TEST_SOCKET to a disposable containerd socket")
	}
	ctx, cancel := context.WithTimeout(namespaces.WithNamespace(context.Background(), fmt.Sprintf("host-ctr-gc-%d", time.Now().UnixNano())), 30*time.Second)
	defer cancel()
	client, err := containerd.New(socket)
	require.NoError(t, err)
	defer client.Close()
	seed, release, err := client.WithLease(ctx)
	require.NoError(t, err)
	t.Cleanup(func() { _ = release(seed) })
	snapshotter := containerd.DefaultSnapshotter
	sn := client.SnapshotService(snapshotter)

	write := func(data []byte, media string, labels map[string]string) oci.Descriptor {
		d := oci.Descriptor{MediaType: media, Digest: digest.FromBytes(data), Size: int64(len(data))}
		require.NoError(t, content.WriteBlob(seed, client.ContentStore(), d.Digest.String(), bytes.NewReader(data), d, content.WithLabels(labels)))
		return d
	}
	// A valid empty tar layer gives both manifests identical cached filesystem data.
	layer := write(make([]byte, 1024), oci.MediaTypeImageLayer, nil)
	_, err = sn.Prepare(seed, "seed", "")
	require.NoError(t, err)
	require.NoError(t, sn.Commit(seed, layer.Digest.String(), "seed"))
	makeImage := func(role string) images.Image {
		labels := map[string]string{}
		if role == "old" {
			labels["containerd.io/gc.ref.snapshot."+snapshotter] = layer.Digest.String()
		}
		cfg, err := json.Marshal(oci.Image{Platform: oci.Platform{Architecture: runtime.GOARCH, OS: "linux"},
			Config: oci.ImageConfig{Labels: map[string]string{"role": role}}, RootFS: oci.RootFS{Type: "layers", DiffIDs: []digest.Digest{layer.Digest}}})
		require.NoError(t, err)
		config := write(cfg, oci.MediaTypeImageConfig, labels)
		body, err := json.Marshal(oci.Manifest{Versioned: spec.Versioned{SchemaVersion: 2}, MediaType: oci.MediaTypeImageManifest, Config: config, Layers: []oci.Descriptor{layer}})
		require.NoError(t, err)
		root := write(body, oci.MediaTypeImageManifest, map[string]string{"containerd.io/gc.ref.content.config": config.Digest.String(), "containerd.io/gc.ref.content.l.0": layer.Digest.String()})
		img, err := client.ImageService().Create(seed, images.Image{Name: "example.invalid/test:" + role, Target: root})
		require.NoError(t, err)
		return img
	}
	makeImage("old")
	next := makeImage("new")
	require.NoError(t, release(seed))
	body, err := content.ReadBlob(ctx, client.ContentStore(), next.Target)
	require.NoError(t, err)
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.URL.Path == "/v2/" {
			w.WriteHeader(http.StatusOK)
			return
		}
		if strings.HasPrefix(r.URL.Path, "/v2/test/manifests/") {
			w.Header().Set("Content-Type", next.Target.MediaType)
			w.Header().Set("Docker-Content-Digest", next.Target.Digest.String())
			w.Header().Set("Content-Length", fmt.Sprint(len(body)))
			w.WriteHeader(http.StatusOK)
			if r.Method != "HEAD" {
				_, _ = w.Write(body)
			}
			return
		}
		http.NotFound(w, r)
	}))
	defer server.Close()
	boundary := &gcBoundary{Snapshotter: sn, client: client, parent: layer.Digest.String()}
	observed, err := containerd.New(socket, containerd.WithServices(containerd.WithSnapshotters(map[string]snapshots.Snapshotter{snapshotter: boundary})))
	require.NoError(t, err)
	defer observed.Close()
	// Explicitly configure the synthetic loopback mirror; production TLS defaults stay intact.
	registry := filepath.Join(t.TempDir(), "registry.toml")
	host := strings.TrimPrefix(server.URL, "http://")
	require.NoError(t, os.WriteFile(registry, []byte(fmt.Sprintf("[mirrors.%q]\nendpoints = [%q]\n", host, server.URL)), 0600))
	_, err = pullImage(ctx, host+"/test:latest", observed, registry, nil)
	require.NoError(t, err)
	require.True(t, boundary.fired, "test must cross the cached-layer GC boundary")
	require.NoError(t, boundary.err)
	// Pull's lease has ended; a second collection must also preserve the new image.
	require.NoError(t, client.ImageService().Delete(ctx, next.Name, images.SynchronousDelete()))
	_, err = sn.Prepare(ctx, "container", layer.Digest.String())
	require.NoError(t, err, "the returned image must remain usable for container creation")
}
