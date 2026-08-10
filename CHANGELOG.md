# Unreleased

# v15.1.0 (2026-08-10)
## OS Changes
* Replace hardcoded ephemeral-storage bind-dir allowlist with package-contributed drop-in fragments in `/usr/lib/bottlerocket/ephemeral-storage.d/` ([#1001])
* Make superpowered containers receive all kernel-known capabilities ([#987]) - Thanks @anirbag

### Third Party Package Updates
* Update `libcrypto`, `libdevmapper`, `libncurses`, `libelf`, and `libffi` ([#998])
* Update `containerd-1.7`, `containerd-2.2`, `kubernetes-1.31`, `kubernetes-1.32`, `kubernetes-1.33`, `kubernetes-1.34`, `kubernetes-1.35`, and `kubernetes-1.36` ([#989])
* Update `aws-signing-helper` ([#989], [#1003])
* Update `soci-snapshotter` ([#989], [#995]) - Thanks @ayush-panta

## Build Changes
* Update Twoliter to `0.22.1` ([#991], [#996])
* Update bottlerocket-sdk to v0.78.0 ([#1002])
* Update first-party tough dependency to the latest version ([#990])

## Orchestrator Changes
* Update eni-max-pods mapping ([#997])

[#987]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/987
[#989]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/989
[#990]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/990
[#991]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/991
[#995]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/995
[#996]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/996
[#997]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/997
[#998]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/998
[#1001]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/1001
[#1002]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/1002
[#1003]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/1003

# v15.0.0 (2026-07-23)

## OS Changes
- Drop `kubernetes-1.30` and `ecr-credential-provider-1.30` packages ([#970])
- Drop `containerd-2.1` package ([#967])

## Build Changes
- Update `bottlerocket-sdk` to v0.77.0 ([#977])

[#967]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/967
[#970]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/970
[#977]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/977

# v14.9.0 (2026-07-20)

## OS Changes
- Add support for static hugepages and transparent hugepages([#952])
- Fix `thar-be-registries` port inference and path validation for registry URLs ([#976])

### Third Party Package Updates
- Update `runc` to 1.3.6 ([#974])
- Update `systemd-257` and backport changes to `systemd-252` ([#961])

## Build Changes
- Bump `bottlerocket-settings-models` to 0.25.0 ([#978])

## Orchestrator Changes
### Kubernetes
- Gate kubelet `--container-runtime-endpoint` on `settings.kubernetes.container-runtime-endpoint` ([#968]) - Thanks @shvbsle!

[#952]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/952
[#961]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/961
[#968]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/968
[#974]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/974
[#976]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/976
[#978]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/978

# v14.8.0 (2026-07-09)

### Third Party Package Updates

- Update `containerd-1.7`, `containerd-2.1` and `containerd-2.2` ([#954])
- Update `aws-signing-helper`, `ecr-credential-provider-1.32-1.35`, `kubernetes-1.30-1.35`, `soci-snapshotter`, `aws-otel-collector` ([#971])
- Update `bash`, `util-linux`, `libglib` ([#973])

[#954]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/954
[#971]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/971
[#973]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/973

# v14.7.0 (2026-07-02)

## OS Changes
- Enable PIE for kmod static builds ([#929])
- Add ghostdog command to create devices ([#963])

### Third Party Package Updates

- Update `kubernetes-1.36` ([#964])
- Backport patch to fix memory leak in `kubernetes-1.36` ([#964])

[#929]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/929
[#963]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/963
[#964]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/964

# v14.6.2 (2026-06-25)

## OS Changes
- Make fips marker file and GO FIPS activation conditional controlled via kernel commandline parameter ([#908])

[#908]:https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/908

# v14.6.1 (2026-06-23)

## OS Changes
- Pin `kmod` binary in `systemd` to use `/usr/bin/` ([#957])

[#957]:https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/957

# v14.6.0 (2026-06-19)

## OS Changes
- Allow disabling `nvidia-k8s-device-plugin` through API ([#914])
- Move FIPS components from `release-fips` sub-package to base package controlled via kernel commandline parameter ([#918])
- Normalize NVIDIA library paths in containers so libraries appear at `/usr/lib` ([#919])
- Update Rust and Go dependencies for first-party sources ([#949])

### Third Party Package Updates
- Update `nvidia-container-toolkit`, `libnvidia-container`, `nvidia-k8s-device-plugin` ([#931])

## Build Changes
- Bump `bottlerocket-settings-models` to 0.24.0 ([#914])
- Update Twoliter to 0.20.0 ([#936])
- Update `bottlerocket-sdk` to v0.76.0 ([#942], [#945])

## Orchestrator Changes
### Kubernetes
- Add the `remap-ids` capability to the soci-snapshotter configuration, allowing it to start pods with `hostUsers: false` ([#939])  - Thanks @mhulscher!

[#914]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/914
[#918]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/918
[#919]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/919
[#931]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/931
[#936]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/936
[#939]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/939
[#942]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/942
[#945]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/945
[#949]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/949

# v14.5.1 (2026-06-18)

### Third Party Package Updates
- Include CVE fixes for containerd-1.7 ([#d3898326])
- Update containerd-2.1 from 2.1.7 to 2.1.8 and include CVE patches ([#6d6e3ba9], [#d05edc04])
- Update containerd-2.2 from 2.2.3 to 2.2.4 and include CVE patches ([#33f6b6d4], [#f45396d9])

[#d3898326]: https://github.com/bottlerocket-os/bottlerocket-core-kit/commit/d38983261fa858d5d21c2fb96d0bee34a30cecf4
[#6d6e3ba9]: https://github.com/bottlerocket-os/bottlerocket-core-kit/commit/6d6e3ba905002d3eed388d0cc54ae5ed172021a3
[#d05edc04]: https://github.com/bottlerocket-os/bottlerocket-core-kit/commit/d05edc045d1abb1987585f2696317c2d25cedb1e
[#33f6b6d4]: https://github.com/bottlerocket-os/bottlerocket-core-kit/commit/33f6b6d40c5db9318fd64306aaa13e57ad5fbd45
[#f45396d9]: https://github.com/bottlerocket-os/bottlerocket-core-kit/commit/f45396d93c2555ed7da51a91d33bfaa6dc1bcd10

# v14.5.0 (2026-06-02)

### Third Party Package Updates
- Update `libaudit`, `liburcu`, `libncurses`, `libglib`, `libbpf`, `libdevmapper`, `libisal`, `libcap`, `libcryptsetup` ([#921])
- Update `e2fsprogs`, `iptable`, `mdadm`, `xfsprogs`, `pciutils`, `util-linux` ([#930])
- Update `kubernetes-1.30-1.35`, `runc`, `ecs-agent`, `amazon-ssm-agent`, `soci-snapshotter`, `cni-plugins` ([#932])
- Update `amazon-ecs-cni-plugins` ([#934])

[#921]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/921
[#930]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/930
[#932]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/932
[#934]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/934

# v14.4.0 (2026-05-26)

### Third Party Package Updates
- Update `libcrypto`, `rdma-core` ([#881])

## Build Changes
- Update Bottlerocket SDK to `v0.74.0` ([#933])

[#881]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/881
[#933]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/933

# v14.3.0 (2026-05-13)

## OS Changes
- Add logging to `pluto` ([#849])
- Configure `containerd` transfer service remote snapshot annotations ([#923])
- Update first-party `tough` dependency to the latest version ([#927])

### Third Party Package Updates
- Update `aws-iam-authenticator`, `aws-signing-helper`, `containerd-1.7`, `containerd-2.1` ([#917])
- Update `containerd-2.2` ([#928])
- Backport sandbox controller fix for `containerd-2.1` and `containerd-2.2` ([#925])

## Orchestrator Changes

### Kubernetes
- Update `kubernetes-1.36`, `ecr-credential-provider-1.36` with official sources ([#920])

[#849]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/849
[#917]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/917
[#920]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/920
[#923]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/923
[#925]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/925
[#927]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/927
[#928]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/928

# v14.2.0 (2026-04-24)

## OS Changes
- Add `max-concurrent-unpacks` setting support for `containerd-2.2` ([#886])
- Update `host-ctr` to use Go `v1.26` ([#896])

### Third Party Package Updates
- Update `binutils` to `v2.44.0` ([#896])
- Update `bash`, `ethtool`, `iproute`, `iptables`, `strace` ([#892])

## Build Changes
- Update Go toolchain to `v1.25` across packages ([#896])
- Update Bottlerocket SDK to `v0.73.0` ([#896])
- Bump `bottlerocket-settings-models` to 0.23.0 ([#901])
- Set build flags for `bash` to follow c17 standard ([#911])
- Configure `systemd-252`, `systemd-257` to set mount and umount path to `%{_cross_bindir})` ([#911])

## Orchestrator Changes

### Kubernetes
- Add beta packages for `kubernetes-1.36` and `ecr-credential-provider-1.36` ([#896])
- Add support for kubernetes settings `prefer-closest-numa-nodes` and `max-allowable-numa-nodes` ([#901])

[#886]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/886
[#892]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/892
[#896]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/896
[#901]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/901
[#911]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/911

# v14.1.0 (2026-04-13)

## OS Changes

### Third Party Package Updates
- Patch `glibc` to revert lazy THP initialization in malloc ([#905])

[#905]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/905

# v14.0.1 (2026-04-09)

## OS Changes
- Allow runtime processes to write fifos to content stores ([#895])
- Add logic to remove orphaned datastores during migration ([#812])
- Update Rust dependencies for first-party sources ([#891])

### Third Party Package Updates
- Update to latest versions of `ecr-credential-provider` ([#876])

[#812]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/812
[#876]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/876
[#891]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/891
[#895]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/895

# v14.0.0 (2026-03-27)

## OS Changes
- Drop `kubernetes-1.29` and `ecr-credential-provider-1.29` packages ([#877])
- Add CDI support to `host-ctr` enabling NVIDIA GPU tools access in superpowered containers ([#879])
- Add `containerd-2.2` package ([#862])
  - Support image volume mount subpath
  - Support parallel unpack during image pull
- Adjust SELinux Policy to limit containers from interacting with the container runtime ([#868])

## Build Changes
- Fix build time race condition in `perf` ([#871])

### Third Party Package Updates
- Update `amazon-ssm-agent` to `3.3.4108` ([#884])

[#862]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/862
[#868]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/868
[#871]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/871
[#877]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/877
[#879]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/879
[#884]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/884

# v13.4.0 (2026-03-24)

## OS Changes
- Add SELinux permissions for kernel 6.18 ([#847])

[#847]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/847

# v13.3.0 (2026-03-18)
## OS Changes
- Reserve EKS add-on ports ([#864]) - Thanks @Shreyank031!

## Build Changes
- Update Bottlerocket SDK to v0.72.0 ([#872])

[#864]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/864
[#872]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/872

# v13.2.0 (2026-03-17)

## OS Changes
- Ensure system services start after `tmp.mount` ([#863])
- Remove `aws-lc-rs` dependency from `shibaken` ([#851])
- Send `whippet` STDERR to both the journal and the console ([#859])
- Override GODEBUG fips140 mode for notation verifier ([#853])
- Update Rust dependencies for first-party sources ([#865], [#866])

### Third Party Package Updates
- Update `nvidia-container-toolkit` patches ([#842])
- Update `ecr-credential-helper` to v0.12.0 ([#846])
- Update `runc` to v1.3.4 ([#854])
- Update `amazon-ecs-cni-plugins` patches ([#855])
- Update `glibc` to v2.43, `libbpf` to v1.6.3, `procps` to v4.0.6, `open-vm-tools` to v13.0.10, `libnetfilter_conntrack` to v1.1.1, `conntrack-tools` to v1.4.9, `coreutils` to v9.10, `hwloc` to v2.13.0, and update `libcrypto`, `libelf`, `libxcrypt` patches ([#865])
- Update `aws-signing-helper` to v1.7.3, `soci-snapshotter` to v0.12.1 ([#850])

## Build Changes
- Update Twoliter to 0.17.0 ([#845])
- Add minimal debuginfo to release builds ([#851])
- Build glibc with minimal debuginfo so that its symbols are available in profilers ([#851])
- Update Bottlerocket SDK to v0.71.0 ([#861])

## Orchestrator Changes
### Kubernetes
- Add `kubelet-env-nvidia` template for `kubernetes-1.35` ([#860])
- Update to latest versions of `kubernetes` packages (1.29-1.35) ([#850])

[#842]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/842
[#845]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/845
[#846]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/846
[#850]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/850
[#851]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/851
[#853]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/853
[#854]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/854
[#855]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/855
[#859]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/859
[#860]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/860
[#861]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/861
[#863]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/863
[#865]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/865
[#866]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/866

# v13.1.2 (2026-03-06)

## OS Changes
### Third Party Package Updates
- Update `amazon-ecs-cni-plugins` ([#855])

[#855]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/855

# v13.1.1 (2026-02-26)

## OS Changes
### Third Party Package Updates
- Update `amazon-ecs-cni-plugins` ([#843])

[#843]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/843

# v13.1.0 (2026-02-25)

## OS Changes
- Suppress IPv6 on interfaces with no IPv6 intent in `net.toml` ([#826])
- Add support to render `settings.container-registry` into containerd supported `hosts.toml` ([#819])
- Expand image verifier support with a new helper to render trust policies for all image verifier plugins ([#820])
- Add `cri-tools`, `erofs-utils`, `perf` packages ([#818])
- Fix ephemeral storage handling in `apiserver` ([#822])
- Stop rendering `max_concurrent_downloads` in containerd CRI image plugin config ([#838])
- Increase `vm.max_map_count` to 1048576 ([#835])
- Update Rust dependencies for first-party sources ([#837])

### Third Party Package Updates
- Update `libnvidia-container`, `nvidia-container-toolkit`, `nvidia-k8s-device-plugin` ([#834])

## Build Changes
- Bump `bottlerocket-settings-models` to 0.21.0 ([#841])

[#819]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/819
[#818]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/818
[#826]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/826
[#820]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/820
[#822]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/822
[#834]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/834
[#835]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/835
[#837]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/837
[#838]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/838
[#841]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/841

# v13.0.0 (2026-02-11)

## OS Changes
- Downgrade `apiserver` dependencies to `Wants` ([#823])
- Drop `containerd-2.0`, `libexpat` packages and `dbus-broker` launcher capability in favor of whippet ([#811])
- Remove `routel` shell script from `iproute` ([#817])
- Add requires to `libbpf` devel package ([#817])
- Use macro for `systemd-257` generators ([#817])
- Remove separate FIPS binaries from Go packages in favor of Go built-in FIPS support ([#813])
- Add URI resolver support to `apiclient apply` and `apiclient network configure` ([#554])
  - s3:// - S3 bucket objects
  - secretsmanager:// - AWS Secrets Manager secrets
  - ssm:// - AWS SSM Parameter Store parameters
  - arn:aws:secretsmanager: and arn:aws:ssm: - cross-region access via full ARN
  - base64: - inline encoded content
  - Default timeouts added for HTTP/HTTPS requests

### Third Party Package Updates
- Update `ecs-agent` to 1.101.2 with matching `amazon-vpc-cni-plugins` ([#816])
- Update `amazon-ecs-cni-plugins` to 2026.01.0 ([#815])
- Update `amazon-ssm-agent`, `aws-otel-collector`, `libcryptsetup`, `libdevmapper`, `libncurses`, `libnl`, `libz`, `libglib`, `mdadm`, `strace`, `util-linux`, `xfsprogs` ([#824])

## Orchestrator Changes
### Kubernetes
- Update latest versions of `kubernetes` packages (1.29-1.34) ([#824])
- Remove hugepages from `reservedMemory` in kubelet config (1.29-1.35) ([#821])
- Update `eni-max-pods` mapping ([#810], [#825])
- Update SELinux policy to allow container communication with MPS daemon ([#831])

### ECS
- Update `ecs-agent` to 1.101.2 ([#816])

[#554]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/554
[#810]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/810
[#811]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/811
[#813]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/813
[#815]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/815
[#816]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/816
[#817]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/817
[#821]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/821
[#823]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/823
[#824]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/824
[#825]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/825
[#831]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/831

# v12.3.0 (2026-01-21)

## OS Changes
- Replace `amazon-ecr-containerd-resolver` with Docker resolver in `host-ctr` ([#760])
- Add MPS control daemon support to `nvidia-k8s-device-plugin` ([#789])
- Add trn3 device ids to `pciclient` ([#800])
- Switch to using Go built-in runtime FIPS support ([#783])

### Third Party Package Updates
- Update `docker-cli-29`, `docker-engine-29` ([#785])
- Patch `containerd-2.1` to update GRPC ([#801])
- Update `libnvme`, `xfsprogs`, `nvme-cli`, `makedumpfile`, `keyutils`, `e2fsprogs` ([#794])
- Update `readline`, `libxcrypt`, `liburcu`, `libcap` ([#795])
- Update `ecr-credential-helper` ([#796])

## Build Changes
- Bump `bottlerocket-settings-models` to 0.20.0 ([#803])
- Update `bottlerocket-sdk` to v0.70.0 ([#783])

## Orchestrator Changes
### Kubernetes
- Add latest instance types to `eni-max-pods` mapping ([#805])

[#760]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/760
[#783]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/785
[#785]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/785
[#789]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/789
[#794]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/794
[#795]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/795
[#796]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/796
[#800]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/800
[#801]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/801
[#803]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/803
[#805]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/805

# v12.2.0 (2026-01-08)

## OS Changes
### Third Party Package Updates
- Update `aws-signing-helper`, `aws-iam-authenticator`, `containerd-1.7`, `containerd-2.1` ([#784])

## Build Changes
- Update `twoliter` to v0.16.0 ([#793])

## Orchestrator Changes
### Kubernetes
* Update `kubernetes-1.35` package with official sources ([#792])
- Update to latest versions of `kubernetes` packages ([#784])

[#784]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/784
[#792]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/792
[#793]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/793

# v12.1.0 (2026-01-02)

## OS Changes
- Add `audit-rules` subpackage to `libaudit` and `journald-audit` subpackage to `systemd` ([#781])
- Add `rocm-container-toolkit` package for AMD GPU support ([#778])
- Override SBOM generation for Rust packages ([#787])

### Third Party Package Updates
- Update `cni-plugins` to v1.9.0 ([#774])
- Update `rocm-k8s-device-plugin` to v1.31.0.9 ([#788])

## Build Changes
- Update `twoliter` to v0.15.1 ([#779])

## Orchestrator Changes
### Kubernetes
- Add `multi-user.target` drop-in for kubelet restarts across all versions ([#773])
- Add `kubernetes-1.35` package with beta source and `ecr-credential-provider-1.35` package with official source ([#777])
- Add latest instance types to `eni-max-pods` mapping ([#776], [#782])

## Documentation
- Remove OCI consideration from BUILDING.md ([#615])

[#615]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/615
[#773]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/773
[#774]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/774
[#776]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/776
[#777]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/777
[#778]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/778
[#779]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/779
[#781]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/781
[#782]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/782
[#787]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/787
[#788]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/788

# v12.0.1 (2025-12-12)

## OS Changes

### Third Party Package Updates
- Revert updates to `libnvidia-container`, `nvidia-container-toolkit`, and `nvidia-k8s-device-plugin` ([#775])

[#775]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/775

# v12.0.0 (2025-12-10)

## OS Changes
- Disable concurrent layer fetch by default in `containerd-2.1` ([#764])
- Add latest instance types to `eni-max-pods` mapping ([#763])
- Update `host-ctr` go dependencies ([#758])
- Update ordering for drivers target to load before settings are applied ([#749])

### Third Party Package Updates
- Update `amazon-ssm-agent`([#768])
- Update core system utilities: `bash`, `chrony`, `coreutils`, `iproute`, `strace`, and `open-vm-tools` ([#765])
- Update multiple core libraries: `libnftnl`, `nftables`, `libpcre`, `libglib`, `libelf`, `libdevmapper`, and `libncurses` ([#767])
- Update `rdma-core` and enable PCI support in `hwloc` ([#725])
- Update `soci-snapshotter` ([#759])
- Update to latest versions of `ecr-credential-provider` and `kubernetes` packages ([#758])
- Update `libnvidia-container`, `nvidia-container-toolkit`, and `nvidia-k8s-device-plugin` ([#758])

## Build Changes
- Update `twoliter` to v0.14.0 ([#765])
- Update `bottlerocket-sdk` to v0.66.0 ([#769])

## Orchestrator Changes
### Kubernetes
- Drop `kubernetes-1.28` and `ecr-credential-provider-1.28` packages ([#761])

[#725]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/725
[#749]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/749
[#758]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/758
[#759]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/759
[#761]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/761
[#763]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/763
[#764]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/764
[#765]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/765
[#767]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/767
[#768]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/768
[#769]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/769

# v11.1.0 (2025-11-26)

## OS Changes
* Provide `libdrm` and `rocm-k8s-device-plugin` packages for AMD GPU detection ([#748])
* Add latest instance types to `eni-max-pods` mapping ([#752])

[#748]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/748
[#752]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/752

# v11.0.1 (2025-11-12)

## Orchestrator Changes
### Kubernetes
- Return `enableDebuggingHandlers` to default behaviour ([#747])

[#747]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/747

# v11.0.0 (2025-11-12)

## OS Changes
- Add image signing verification for ECR images signed by AWS Signer ([#722])
- Add an apiclient command to lockdown the datastore to prevent further changes ([#727])
- Provide `rottweiler`, a unified storage encryption helper ([#717])
- Add support for encrypted storage ([#721])
- Fix `whippet` defaults and wildcard replacements ([#720])
- Add apiclient support to exclude settings prefixes and canonicalize output ([#716])
- Add `apiclient network configure` subcommand ([#714])
- Ensure that bootconfig keys are written in a consistent order ([#735])
- Enhance `bloodhound` CIS compliance checks ([#665], [#738])
- Decouple the network stack initialization from the DATA partition ([#638])
- Add EBS volumes support for ephemeral storage ([#395]) - Thanks @jesseanttila-cai
- Build `systemd-257` with `cryptsetup` support ([#691])
- Update `host-ctr` go dependencies ([#723])
- Build `libcryptsetup` and `libdevmapper` with udev support ([#706])
- Support kdump for zboot kernels on aarch64 ([#707])

### Third Party Package Updates
- Add `hwloc` package ([#672])
- Update `systemd-252` to v252.39 ([#700])
- Update `systemd-257` to v257.9 ([#691])
- Drop `socat` package ([#742])
- Update `libexpat` ([#695])
- Add `libudev` package ([#706])
- Update `kexec-tools` ([#707])
- Add `docker-cli-29`, `docker-engine-29` packages ([#711], [#743], [#745])
- Update `aws-otel-collector`, `aws-signing-helper` ([#715])
- Update `containerd-1.7`, `containerd-2.0`, `containerd-2.1` ([#724])

## Build Changes
- Add changelog validation improvements ([#699])
- Update `bottlerocket-settings-models` to v0.17.0 ([#689])
- Update `twoliter` from v0.12.0 to v0.13.0 ([#736])

## Orchestrator Changes
### Kubernetes
- Update DNS IP generation to support IPv6 ([#734])
- Update to latest versions of `ecr-credential-provider` and `kubernetes` packages ([#715])
- Add `enableDebuggingHandlers`, `imageMinimumGCAge`, `maxParallelImagePulls`, `ImageMaximumGCAge` and CPU manager settings ([#689])

### ECS
- Default to containerd's transfer service for `docker-engine-29` ([#730])

[#395]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/395
[#638]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/638
[#665]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/665
[#672]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/672
[#689]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/689
[#691]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/691
[#695]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/695
[#699]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/699
[#700]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/700
[#706]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/706
[#707]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/707
[#711]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/711
[#714]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/714
[#715]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/715
[#716]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/716
[#717]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/717
[#720]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/720
[#721]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/721
[#722]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/722
[#723]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/723
[#724]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/724
[#727]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/727
[#730]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/730
[#734]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/734
[#735]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/735
[#736]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/736
[#738]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/738
[#742]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/742
[#743]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/743
[#745]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/745

# v10.9.3 (2025-11-11)

## Orchestrator Changes
### Kubernetes
- Patch `ecr-credential-provider` to support AWS EUSC ([#729])

[#729]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/729

# v10.9.2 (2025-11-08)

### Third Party Package Updates
- Patch runc to set the correct mode for tmpfs mounts ([#731])

[#731]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/731

# v10.9.1 (2025-11-05)

### Third Party Package Updates
- Update runc to v1.2.8 ([#708])

[#708]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/708

# v10.9.0 (2025-11-05)

## OS Changes
* Update runc to v1.2.7 and include CVE patches ([#6813a59b], [#6e3d3e2e], [#f330515a])
* containerd-2.1: fix image pull error when range-get request is ignored ([#702])

[#6813a59b]: https://github.com/bottlerocket-os/bottlerocket-core-kit/commit/6813a59bb4b681239f991217e835e43836242b32
[#6e3d3e2e]: https://github.com/bottlerocket-os/bottlerocket-core-kit/commit/6e3d3e2e563ec556b9fc51eb495a180b69bcf43b
[#f330515a]: https://github.com/bottlerocket-os/bottlerocket-core-kit/commit/f330515aacc658abdcd4c86b07b6e3ae22de8d13
[#702]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/702

# v10.8.1 (2025-10-22)

## Build Changes
* Update `bottlerocket-sdk` from 0.65.0 to 0.65.1 ([#698])

[#698]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/698

# v10.8.0 (2025-10-15)

## OS Changes
* Provide command to detect first and third-party accelerated hardware ([#644])
* Provide `whippet` as an alternative `dbus-launcher` ([#661], [#678])
* Make `dbus-broker` require `dbus-launcher` capability ([#677])
* Provide `dbus-broker-launcher` as a separate package ([#677])
* Allow multiple sequential calls of `apiclient ephemeral-storage bind` ([#679])
* Log pending settings only for `debug` or higher log levels ([#690]) - Thanks @fletcherw

## Build Changes
* Update `bottlerocket-sdk` from 0.64.0 to 0.65.0 ([#684])
* Fix clippy warnings for Rust 1.90.0 ([#684])

### Third Party Package Updates

- Update `aws-iam-authenticator`, `aws-ssm-agent` ([#684], [#688])

[#644]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/644
[#661]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/661
[#677]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/677
[#678]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/678
[#679]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/679
[#684]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/684
[#688]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/688
[#690]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/690

# v10.7.1 (2025-10-06)

### Third Party Package Updates

- Add a patch for `libnvidia-container` to support glibc ([#687])

[#687]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/687

# v10.7.0 (2025-10-02)

## OS Changes
- Update Rust dependencies for first-party sources ([#673])
- Update Go dependencies for first-party sources ([#673])
- Patch `systemd` to suppress a warning log that is not applicable to Bottlerocket ([#681])

### Third Party Package Updates
- Update `glibc` and `docker-engine` ([#676], [#671])
- Update core libraries: `libpcre`, `readline`, `libz`, `libtirpc`, `libnftl`, `libbncurses`, `libinih`, `libglib`, `libffi`, `libbpf`, `libdevmapper`, and `libscrypsetup` ([#683])
- Update core system utilities: `iproute`, `strace`, `nvme-cli`, `libnvme`, `xfsprogs`, `ethtool`, `util-linux`, `pciutils`, `dbus-broker`, and `e2fsprogs`([#675], [#680])

## Build Changes
- Update `bottlerocket-settings-models` to v0.16.0 ([#646])

## Orchestrator Changes
### Kubernetes
- Add `pid` resource to `kubeReserved` setting ([#646])

[#646]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/646
[#671]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/671
[#673]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/673
[#675]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/675
[#676]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/676
[#680]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/680
[#681]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/681
[#683]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/683

# v10.6.0 (2025-09-23)

## OS Changes
- Update ECR parsing in `host-ctr` after `aws-sdk-go-v2` migration ([#664])

[#664]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/664

# v10.5.0 (2025-09-22)

## OS Changes
- Support arguments with a `--` separator in `apiclient exec` subcommand ([#647])
- Backport `systemd` patch to suppress `ENOENT` error logs ([#655])
- Install `driverdog` for all variants ([#656]) - Thanks @fletcherw

### Third Party Package Updates
- Update `libexpat`, `aws-iam-authenticator`, `containerd-1.7`, `containerd-2.0`, `kubernetes-1.28-1.34` ([#663], [#666])

[#647]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/647
[#655]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/655
[#656]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/656
[#663]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/663
[#666]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/666

# v10.4.1 (2025-09-11)

## Build Changes
* Update `bottlerocket-settings-models` to 0.15.0 ([#658])

[#658]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/658

# v10.4.0 (2025-09-08)

## OS Changes
* Add command field to override default entrypoint for host and bootstrap containers ([#594]) - Thanks @kasimeka
* Update `systemd-257` to remove shutdown timeout patch, migrate mount-rate patch to bootconfig, and remove kernel cmdline requirement for `cgroupsv1` ([#636])
* Add `containerd-2.1` setting for `concurrent-download-chunk-size` ([#645])
* Add support for more AWS regions in `schnauzer` and `host-ctr` ([#651])

## Build Changes
* Update `bottlerocket-settings-models` to 0.14.0 ([#645])

## Orchestrator Changes
### Kubernetes
* Update `kubernetes-1.34` and `ecr-credential-provider-1.34` packages with official sources ([#653])

[#594]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/594
[#636]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/636
[#645]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/645
[#651]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/651
[#653]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/653

# v10.3.0 (2025-08-26)

## OS Changes
- Add default bind directories for ephemeral storage ([#632])
- Extend netdog to look for `net.toml` under `/usr/share/bottlerocket` ([#524]) - Thanks @pb80
- Add `containerd-2.1` package ([#621])
  - Transfer service for image pull is now the default
  - Multipart layer fetch support was added and has a default of 8MiB in Bottlerocket
  - Containerd 2.1 removes the support for Schema 1 images
- Add `systemd-257` package ([#581])
- Update `host-ctr` to migrate to `aws-sdk-go-v2` and bump to go 1.24 ([#642])

## Orchestrator Changes
### Kubernetes
- Enable `MutableCSINodeAllocatableCount` feature gate on kubelet for kubernetes-1.34 ([#634]) - Thanks @torredil
- Add support for new Kubernetes Setting `static-pods-enabled` ([#641])

## Build Changes
- Update `twoliter` to 0.12.0 ([#635])
- Update bottlerocket-settings-models to 0.13.0 ([#641])

[#524]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/524
[#581]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/581
[#621]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/621
[#632]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/632
[#634]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/634
[#635]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/635
[#641]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/641
[#642]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/642

# v10.2.0 (2025-08-19)

## Orchestrator Changes
### Kubernetes
* Add kubernetes-1.34 and ecr-credential-provider-1.34 packages with pre-release sources ([#627])

## Build Changes
* Update bottlerocket-sdk from 0.63.0 to 0.64.0 ([#629])

[#627]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/627
[#629]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/629

# v10.1.2 (2025-08-14)

### Third Party Package Updates
- Revert `ecs-agent` update ([#625])

[#625]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/625

# v10.1.1 (2025-08-13)

## OS Changes
* Fix `containerd-2.0` settings for `max_concurrent_downloads` ([#623])

[#623]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/623

# v10.1.0 (2025-08-11)

## OS Changes
- Add `libjansson` package to enable JSON support for nftables ([#614])
- Refactor `schnauzer` to multicall binary for v1 and v2 ([#561])
- Fix `logind` service ordering in release package ([#609]) - Thanks @115100
- Lowercase hostnames provided by the hostname helpers ([#619]) - Thanks @tzneal

### Third Party Package Updates
- Update `amazon-ssm-agent`, `docker-engine`, and `ecs-agent` packages ([#616])
- Update to latest versions for `aws-iam-authenticator`, `aws-otel-collector`, `aws-signing-helper`, `nvidia-k8s-device-plugin`, `ecr-credential-provider`, and `kubernetes` packages ([#611])

## Orchestrator Changes

### ECS
- Fix ECS_DISABLE_PRIVILEGED in `ecs-agent`([#610]) - Thanks @vermdeep

## Build Changes
- Inject a trait into check execution to allow unit testing ([#601]) - Thanks @tzneal

## Tools
- Add Amazon Q development rules and Git formatting guidelines ([#561])

[#561]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/561
[#601]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/601
[#609]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/609
[#610]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/610
[#611]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/611
[#614]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/614
[#616]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/616
[#619]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/619

# v10.0.1 (2025-07-31)

## Orchestrator Changes
### Kubernetes
- Backport a patch to fix kubelet drop-in config merge behavior in kubernetes-1.28 ([#613])

[#613]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/613

# v10.0.0 (2025-07-25)

## OS Changes
- Deprecate wicked package ([#560])
- Fix file descriptor leak in `apiserver exec` ([#595])
- Add release subpackage to enable zram-backed `swap` ([#590])

### Third Party Package Updates
- Update `cni`, `cni-plugins`, `libaudit`, `libbpf`, `libdevmapper`, `libglib`, and `libncurses` ([#600])

## Orchestrator Changes
### Kubernetes
- Add soci-snapshotter support
  - Configure soci-snapshotter for parallel pull unpack feature ([#569])
  - Optionally configure containerd and kubelet with soci-snapshotter via drop-in configuration files ([#576])
  - Extend selinux-policy to cover soci-snapshotter ([#579])
  - Add `configure-snapshotter.service` to reset state directories of snapshotters on boot when selected snapshotter changes ([#582])
  - Apply upstream patches to soci-snapshotter ([#599])
  - Drop CLI from `soci-snapshotter` ([#569])
- Support extending kubelet configuration via drop-in files ([#576])
- Update to the latest CIS K8s guidance v1.11.1 ([#563]) - Thanks @tzneal
- Drop `kubernetes-1.27` and `ecr-credential-provider-1.27` ([#605])

## Build Changes
- Update `twoliter` to 0.11.0 ([#592])

[#560]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/560
[#563]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/563
[#569]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/569
[#576]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/576
[#579]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/579
[#582]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/582
[#590]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/590
[#592]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/592
[#595]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/595
[#599]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/599
[#600]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/600
[#605]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/605

# v9.2.1 (2025-07-24)

## OS Changes
* Add latest instance types to `eni-max-pods` mapping ([96d45ad5])

[96d45ad5]: https://github.com/bottlerocket-os/bottlerocket-core-kit/commit/96d45ad5348c593978ed08bf354c0e10fc49e8b0

# v9.2.0 (2025-07-16)

## OS Changes
* Namespace `systemd` to `systemd-252` ([#537])
* Pass proxy environment variables to bootstrap-containers ([#564]) - Thanks @abhay-krishna
* Pass proxy environment variables to the soci-snapshotter service ([#584])
* Allowlist `soci-snapshotter` paths to ephemeral storage ([#571])
* Add  `nftables` and `iptables-nft` ([#549])
* Enable support for SELinux efficient relabling ([#573])

### Third Party Package Updates
- Update `kmod` ([#562])
- Update `soci-snapshotter` ([#565])
- Update `xfsprogs` and `chrony` ([#577])

## Orchestrator Changes
### Kubernetes
* Enable DynamicResourceAllocation feature gate on kubelet for k8s-1.33 ([#567])

## Build Changes
* Update bottlerocket-sdk from 0.62.0 to 0.63.0 ([#587])

[#537]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/537
[#549]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/549
[#562]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/562
[#564]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/564
[#565]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/565
[#567]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/567
[#571]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/571
[#573]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/573
[#577]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/577
[#584]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/584
[#587]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/587

# v9.1.0 (2025-06-23)

## OS Changes
* Add an option to write settings once during boot ([#548])

### Third Party Package Updates
- Update `libcrypto` patches ([#546])
- Update to latest versions of kubernetes packages ([#551])
- Update `nvidia-container-toolkit` and `libnvidia-container` to 1.17.8 ([#552])
- Update core system utilities: `kexec-tools`, `open-vm-tools`, and `iputils` ([#553])

## Orchestrator Changes
### Kubernetes
* Add support for Kubernetes setting `memory-swap-behaviour` ([#541]) Thanks @teskje

[#541]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/541
[#546]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/546
[#548]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/548
[#551]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/551
[#552]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/552
[#553]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/553

# v9.0.0 (2025-06-10)

## OS Changes
* Add support for default configuration file in `xfsprogs` ([#521])
* Add support for more AWS regions in `schnauzer` and `host-ctr` ([#535])
* Backport systemd fix to speed up `systemctl daemon-reload` ([#528])
* Replace `systemctl isolate` with `systemctl start` ([#528])
* Add capability markers `ioctl_skip_cloexec` and `userspace_initial_context` to SELinux policy ([#534])
* Add `zramctl` to `util-linux` package ([#543])
### Third Party Package Updates
- Update to latest versions of `aws-iam-authenticator`, `ecr-credential-provider`, and `kubernetes` packages ([#531])
- Update multiple core libraries: SELinux components, `libseccomp`, `libinih`, `libffi`, `libnftnl`, `libelf`, `liburcu`, `libglib`, and `libcap` ([#515])
- Update core system utilities: `ethtool`, `grep`, `iproute`, `strace`, `makedumpfile`, `nvme-cli`, and `libnvme` ([#532])

## Build Changes
* Fix clippy warnings for Rust 1.87.0 ([#525])
* Build `glibc` with frame pointers ([#527])
* Fix various build warnings and LTO compatibility issues in packages ([#526])
* Update `bottlerocket-sdk` from 0.61.0 to 0.62.0 ([#542])

## Orchestrator changes

### Kubernetes
* Drop `kubernetes-1.26` and `ecr-credential-provider-1.26` ([#523])
* Make `soci-snapshotter` a socket-activated systemd service ([#529])
* Add latest instance types to `eni-max-pods` mapping ([#538])

[#515]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/515
[#521]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/521
[#523]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/523
[#525]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/525
[#526]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/526
[#527]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/527
[#528]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/528
[#529]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/529
[#531]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/531
[#532]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/532
[#534]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/534
[#535]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/535
[#538]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/538
[#542]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/542
[#543]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/543

# v8.2.0 (2025-05-20)

## OS Changes
* Third party package updates ([#494], [#498], [#513], [#514])
* Extend `ghostdog` for Infiniband detection and configuration ([#499])
* Enable `cryptsetup` and `tpm2` functionality for systemd ([#518])

## Build Changes
* Update `twoliter` from 0.9.0 to 0.10.1 ([#491], [#509])
* Update bottlerocket-settings-models to 0.10.0 ([#520])

## Orchestrator changes
### ECS
* Migrate ECS to use CDI ([#482])

### Kubernetes
* Support CDI and legacy NVIDIA Container Runtime modes ([#459], [#500], [#507], [#511])
* Patch `nvidia-k8s-device-plugin` to add ldcache parsing ([#501])
* Apply upstream patches for EKS 1.26 ([#517])

[#459]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/459
[#482]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/482
[#491]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/491
[#494]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/494
[#498]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/498
[#499]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/499
[#500]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/500
[#501]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/501
[#507]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/507
[#509]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/509
[#511]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/511
[#513]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/513
[#514]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/514
[#517]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/517
[#518]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/518
[#520]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/520

# v8.1.1 (2025-05-14)

## OS Changes
* Fix `containerd-2.0` settings for `container-registry` ([#504])

[#504]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/504

# v8.1.0 (2025-05-05)

## OS Changes
* Add `containerd-2.0` package ([#485])
* Update `containerd-1.7` CRI spec to match upstream ([#485])
* Update `containerd-1.7` service with `OOMScoreAdjust` to match upstream ([#485])

## Orchestrator Changes
### Kubernetes
* Add support for more Kubernetes Settings ([#487], [#489])
  * `containerLogMaxWorkers`
  * `containerLogMonitorInterval`
  * `singleProcessOOMKill`
* Update `kubernetes-1.33` and `ecr-credential-provider-1.33` packages with official sources ([#488])

[#485]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/485
[#487]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/487
[#488]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/488
[#489]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/489

# v8.0.0 (2025-04-28)

## OS Changes
* Update `host-ctr` dependencies ([#475])
* Add support for GRID drivers ([#483])

## Build Changes
* Update `twoliter` from 0.8.1 to 0.9.0 ([#478])

## Orchestrator Changes
### Kubernetes
* Drop `kubernetes-1.25` and `ecr-credential-provider-1.25` ([#484])

[#475]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/475
[#478]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/478
[#483]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/483
[#484]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/484

# v7.0.1 (2025-04-22)

## Orchestrator Changes
### ECS
* Revert CDI migration for ECS ([#480])

[#480]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/480

# v7.0.0 (2025-04-18)

## OS Changes
* Fix aws-signing-helper and IAM Roles Anywhere ([#451])
* Clear configuration-files and services in migrator ([#456])
* Drop shimpei and oci-add-hooks packages ([#458])
* Restrict kernel dump collection to x86_64 ([#465])
* Third party package updates ([#469], [#472])

## Orchestrator Changes
### Kubernetes
* Apply upstream patches for EKS 1.25-1.27 ([#472])
* Let kubelet start when swap is on ([#473])
* Add kubernetes-1.33 and ecr-credential-provider-1.33 packages with pre-release sources ([#476])

### ECS
* Migrate ECS to use CDI ([#471])

## Build Changes
* Update bottlerocket-sdk from 0.60.0 to 0.61.0 ([#449], [#474])

[#449]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/449
[#451]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/451
[#456]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/456
[#458]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/458
[#465]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/465
[#469]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/469
[#471]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/471
[#472]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/472
[#473]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/473
[#474]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/474
[#476]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/476

# v6.3.0 (2025-04-11)

## OS Changes
* Add cryptsetup package and its dependencies ([#444]) - Thanks @mikn !
* Third party package updates ([#457])
* Update runc from 1.1.15 to 1.2.6 ([#463])
* Allow lookups of `.local` domains using unicast DNS ([#464]) - Thanks @tzneal !

## Orchestrator Changes
### Kubernetes
* Update EKS 1.28-1.32 versions to latest ([#457])

## Build Changes
* Update rust build dependencies ([#457], [#460], [#462])

[#444]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/444
[#457]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/457
[#460]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/460
[#462]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/462
[#463]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/463
[#464]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/464

# v6.2.0 (2025-04-01)

## OS Changes
* Update readline from 8.2 to 8.2.13 ([#446])
* Update glibc from 2.40 to 2.41 ([#437])
* Fix usage of `/var/run` in mdadm tmpfiles snippet ([#442])
* Refactor systemd to explicitly list packaged files ([#438])
* Switch to igzip (x86_64) or pigz with zlib-ng (aarch64) to decompress container images ([#443])
* Add support for more AWS regions in schnauzer and host-ctr ([#454])

## Build Changes
* Remove bottlerocket-variant crate ([#435])

[#435]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/435
[#437]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/437
[#438]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/438
[#442]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/442
[#443]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/443
[#446]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/446
[#454]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/454

# v6.1.1 (2025-03-24)

## OS Changes
* Third party package updates ([#432], [#434])
* Update host-ctr dependencies ([#431])
* Update containerd from 1.7.26 to 1.7.27 ([#434])

## Build Changes
* Update `twoliter` from 0.8.0 to 0.8.1 ([#428])

## Orchestrator Changes
### Kubernetes
* Apply upstream patches for EKS 1.25-1.26 ([#434])
* Update ecr-credential-provider 1.29-1.32 to latest ([#434])
* Add ecr-credential-provider 1.26 and 1.28 ([#434])

[#428]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/428
[#431]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/431
[#432]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/432
[#434]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/434

# v6.1.0 (2025-03-14)

## OS Changes
* Persist sysctl setting changes to /etc/sysctl.d ([#333]) - Thanks @aetimmes
* Updated cis report to account for formatting change in iptables ([#390])
* Update MIG template to handle the no-default case ([#399])
* Third party package updates ([#365], [#371], [#383], [#384], [#403], [#404], [#406])

## Build Changes
* Update `bottlerocket-sdk` from 0.50.1 to 0.60.0 ([#375], [#402])
* Update `twoliter` from 0.7.3 to 0.8.0 ([#368], [#398])

## Orchestrator Changes

### Kubernetes
* Apply upstream patches for Kubernetes 1.25-1.32 ([#379], [#400])

[#333]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/333
[#365]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/365
[#368]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/368
[#371]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/371
[#375]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/375
[#379]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/379
[#383]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/383
[#384]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/384
[#390]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/390
[#398]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/398
[#399]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/399
[#400]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/400
[#402]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/402
[#403]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/403
[#404]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/404
[#406]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/406

# v6.0.2 (2025-02-26)

## Build Changes
* Update `twoliter` from 0.6.0 to 0.7.3 ([#382])

[#382]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/382

# v6.0.1 (2025-02-14)

## OS Changes
* Update `containerd` from 1.7.24 to 1.7.25 ([#374])
* Patch`containerd` 1.7.25 to address issues in runc shim and CRI plugin ([#374])

[#374]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/374

# v6.0.0 (2025-02-07)

## OS Changes
* Add the ability to run custom transaction checks when committing transactions to Bottlerocket’s datastore ([#294])
* Add support for `weak` settings values, which are deleted on update ([#294])
* Only return `strong` settings-generators from the apiserver’s /metadata/settings-generator route ([#294])
* Always delete and re-populate metadata on first boot or update ([#294])
* Add support for dynamic settings-generators via the depth attribute ([#294])
* Add NVIDIA Multi-Instance GPU (MIG) settings to nvidia-k8s-device-plugin ([#258])
* Conditionalize source and mode in Bootstrap container template ([#335])
* Update host-ctr dependencies ([#337])
* Accept comment lines in boot config ([#361])

## Orchestrator Changes

### Kubernetes
* Drop Kubernetes 1.24 variants ([#364])

[#258]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/258
[#294]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/294
[#335]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/335
[#337]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/337
[#361]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/361
[#364]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/364

# v5.4.2 (2025-01-24)

## OS Changes

* Update nvidia-container-toolkit and libnvidia-container to v1.17.4 ([#358])

## Build Changes

* Update Bottlerocket SDK to v0.50.1 ([#345])

## Orchestrator Changes

### Kubernetes
* Update EKS 1.28-1.31 versions to latest ([#356])

[#358]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/358
[#345]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/345
[#356]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/356

# v5.4.1 (2025-01-16)

## OS Change
* Parse proxy URI after prepending URL scheme ([#339])
* Normalize inputs for ephemeral-storage ([#350])

[#339]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/339
[#350]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/350

# v5.4.0 (2025-01-14)

## OS Change
* Allow bind mounts prefixed with /mnt/ for ephemeral storage ([#320]) - Thanks @zaheerm!
* Improve API Server error message for invalid metadata ([#342])

## Orchestrator Changes

### Kubernetes
* Add support for `device-ownership-from-security-context` to nvidia ([#343])

### ECS
* Update `ecs-agent` to 1.89.2 and update the `amazon-ecs-cni-plugins` ([#341])

[#320]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/320
[#341]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/341
[#342]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/342
[#343]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/343

# v5.3.0 (2025-01-03)

## Orchestrator Changes

### Kubernetes
* Add Kubernetes 1.32 packages ([#334])
* Add support for `device-ownership-from-security-context` ([#329])

## Build Changes
* Update bottlerocket-settings-models to 0.7.0 ([#329])

[#334]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/334
[#329]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/329


# v5.2.0 (2024-12-20)

## OS Changes
* Third party package updates ([#322], [#323], [#324], [#325], [#328])

## Build Changes
* Add GPG verification where possible ([#321])

[#321]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/321
[#322]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/322
[#323]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/323
[#324]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/324
[#325]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/325
[#328]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/328

# v5.1.0 (2024-12-16)

## OS Changes

* Update `golang.org/x/crypto` from 0.27.0 to 0.31.0 in /sources/host-ctr ([#315])

## Orchestrator Changes

### Kubernetes
* Add beta sources for kubernetes-1.32 and ecr-credential-provider-1.32 ([#317])

[#315]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/315
[#317]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/317

# v5.0.0 (2024-12-12)

## OS Changes
* Update binutils to 2.41 ([#306])

## Build Changes
* Update bottlerocket-sdk to v0.5.0 ([#306])
* Remove the following packages and migrate them to the [kernel-kit](https://github.com/bottlerocket-os/bottlerocket-kernel-kit) ([#313])
  * grub
  * kernel-5.10
  * kernel-5.15
  * kernel-6.1
  * kmod-5.10-nvidia
  * kmod-5.15-nvidia
  * kmod-6.1-nvidia
  * libkcapi
  * linux-firmware
  * microcode
  * shim

[#306]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/306
[#313]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/313

# v4.1.0 (2024-12-10)

## OS Changes
* Enable plugins and detailed EBS volume stats for `nvme-cli` ([#269])
* Set `LoaderTimeInitUSec` and `LoaderTimeExecUSec` in GRUB ([#273])
* Third party package updates ([#303], [#308], [#311])
* Update kernel to v6.1.119 ([#309])

## Build Changes
* Update twoliter to 0.6.0 ([#302])

[#269]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/269
[#273]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/273
[#302]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/302
[#303]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/303
[#308]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/308
[#309]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/309
[#311]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/311

# v4.0.1 (2024-12-05)

## OS Changes
* Normalize `amazon-ecs-cni-plugins` version ([#277])
* Add host certs to host containers using a volume mount ([#278])
* Fix `host-ctr` to correctly extract regions from ECR URIs ([#287])
* Run udev after the SELinux Policy files are available ([#290])
* Update `nvidia-container-toolkit` and `libnvidia-container` ([#296])

[#277]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/277
[#278]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/278
[#287]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/287
[#290]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/290
[#296]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/296

# v4.0.0 (2024-11-20)

## OS Changes
* Disable BPF preload and bpfilter helpers for kernel ([#261])
* Allow overriding max-pods file with one from variant ([#279])
* Update libdbus and libexpat ([#270])
* Remove acpid package ([#280])
* Prevent io_uring calls from hanging ([#284])

## Orchestrator Changes
### Kubernetes
* Update EKS 1.28-1.31 versions to latest ([#281])

## Build Changes
* Use upstream sources for packages sourced from Amazon Linux ([#265])
* Update twoliter to v0.5.1 ([#259])
* Drop "dependencies" table for all packages ([#262])

[#259]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/259
[#261]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/261
[#262]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/262
[#265]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/265
[#270]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/270
[#279]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/279
[#280]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/280
[#281]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/281
[#284]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/284

# v3.3.2 (2024-11-15)

## OS Changes
* Add kernel-5.15 patch to fix IPv6 typo ([#266])

[#266]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/266

# v3.3.1 (2024-11-14)

## OS Changes
* Update kernel 5.10.228 and kernel 6.1.115 ([#263])

[#263]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/263

# v3.3.0 (2024-11-12)

## Orchestrator Changes
### Kubernetes
* Add latest instance types to eni-max-pods mapping ([#250])

## OS Changes
* Include `rdma-core` in AWS variants ([#252])
* Add `libstdc++` subpackage to `libgcc` ([#253])
* Update third-party packages ([#254])

[#250]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/250
[#252]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/252
[#253]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/253
[#254]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/254

# v3.2.0 (2024-11-06)

## Orchestrator Changes
### Kubernetes
* Match the EKS Optimized AMIs secure TLS ciphers ([#230], [#245])

## OS Changes
* Provide FIPS binaries for first-party rust programs ([#173])
* Support ECR FIPS endpoints for host containers ([#204])
* Adjust SELinux Policy to allow execute programs in NFS filesystems ([#205])
* Mount `binfmt_mics` filesystem at boot ([#206])
* Add proxy support for pluto FIPS binary ([#213])
* Generate default AWS config file ([#218])
* Adjust SELinux Policy for first-party FIPS rust programs ([#222])
* Update third-party packages ([#210], [#212], [#214], [#219], [#220], [#246])
* Add `rdma-core` to packages ([#223])
* Use Amazon Linux 2023 as upstream for `libkcapi` ([#224])
* Set AWS_SDK_LOAD_CONFIG for system services ([#243])
* Add proxy support for `cfsignal` ([#234])

## Build Changes
* Build Neuron kernel module in kernel packages ([#207])
* Update Bottlerocket SDK to v0.47.0 ([#241])

## Tools
* Add `insertFinalNewline` for VSCode Workspaces ([#242])

[#173]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/173
[#204]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/204
[#205]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/205
[#206]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/206
[#207]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/207
[#210]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/210
[#212]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/212
[#213]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/213
[#214]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/214
[#218]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/218
[#219]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/219
[#220]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/220
[#222]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/222
[#223]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/223
[#224]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/224
[#230]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/230
[#234]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/234
[#241]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/241
[#242]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/242
[#243]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/243
[#245]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/245
[#246]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/246

# v3.1.5 (2024-11-04)

## OS Changes
* Wait for kubelet device-manager socket before starting nvidia-k8s-device-plugin ([#228])

[#228]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/228

# v3.1.4 (2024-11-01)

## OS Changes
* Update kernel 5.10.227 and kernel 5.15.168 ([#235])

[#235]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/235

# v3.1.3 (2024-10-31)

## OS Changes
* Update kernel 6.1.112-124 ([#231])

[#231]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/231

# v3.1.2 (2024-10-30)

## OS Changes
* Add kernel-6.1 patch to fix io statistics for cgroup v1 ([#225])

[#225]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/225

# v3.1.1 (2024-10-24)

## OS Changes
* Revert system-wide configuration to block writeable/executable memory in systemd services ([#215])

[#215]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/215

# v3.1.0 (2024-10-22)

## OS Changes
* Update NVIDIA driver versions to 535.216.01 ([#209])

## Build Changes
* Set Epoch to 1 in necessary packages ([#208])

## Orchestrator Changes

### Kubernetes
* Apply upstream patches for Kubernetes v1.24 to v1.31 ([#186])

[#186]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/186
[#208]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/208
[#209]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/209

# v3.0.0 (2024-10-17)

## OS Changes
* Drop kubernetes-1.23 ([#184])
* Move kmod-5.10-nvidia from branch R470 to R535 ([#181])
* Block writeable/executable memory in systemd services by default ([#158])

## Build Changes
* Update twoliter to 0.5.0 ([#195])
* Update bottlerocket-sdk to 0.46.0 ([#191])
* Update `tough` and `reqwest` to latest versions ([#197])
* Set Epoch to 1 in necessary packages ([#180])
* Drop dependency on glibc for nvidia kmods ([#194])

[#158]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/158
[#180]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/180
[#181]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/181
[#184]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/184
[#191]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/191
[#194]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/194
[#195]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/195
[#197]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/197

# v2.9.1 (2024-10-16)

## OS Changes
* Update kernels to 5.10.226, 5.15.167 and 6.1.112 ([#200])


[#200]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/200

# v2.9.0 (2024-10-09)

## OS Changes
* Add nvidia-persistenced and nvidia-modprobe to kmod-*-nvidia ([#122])
* Add NVIDIA time-slicing settings to nvidia-k8s-device-plugin ([#169])
* libcap: fix cross-compile toolchain usage ([#174])
* login: start the getty services earlier ([#175])
* Update amazon-ssm-agent to v3.3.987.0 ([#182])

## Build Changes
* Update twoliter to 0.4.7 ([#183])
* Update bottlerocket-settings-models to 0.6.0 ([#169])

[#122]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/122
[#169]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/169
[#174]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/174
[#175]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/175
[#182]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/182
[#183]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/183

# v2.8.4 (2024-10-03)

## OS Changes
*  Update kernels to 5.10.226 and 5.15.167 ([#177])

[#177]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/177

# v2.8.3 (2024-10-02)

## OS Changes
* Update ecs-agent to v1.86.3 ([#168])
* Update kmod-6.1-neuron to 2.18.12.0 ([#170])

## Build Changes
* Exclude more object files from kernel-6.1-devel ([#172])

[#168]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/168
[#170]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/170
[#172]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/172

# v2.8.2 (2024-09-28)

## OS Changes
* Fix driver unit dependencies for ecs-gpu-init ([#166])

[#166]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/166

# v2.8.1 (2024-09-27)

## Build Changes
* Install squashed kernel-devel if erofs is not positively selected ([#163])

[#163]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/163

# v2.8.0 (2024-09-26)

## Build Changes
* Choose the correct checksum when validating a twoliter binary ([#157])

## OS Changes
* Use open GPU drivers on P4 and P5 instances ([#114])
* Add package-level support for EROFS as a root filesystem ([#159])
* Update libnvidia-container and nvidia-container-toolkit to 1.16.2 ([#161])

[#114]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/114
[#157]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/157
[#159]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/159
[#161]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/161

# v2.7.0 (2024-09-19)

## Build Changes
* Update twoliter to 0.4.6 ([#153])

## OS Changes
* Add the ability for ghostdog to detect EFA devices attachment ([#141])
* Apply higher MEMLOCK limits in oci-defaults when EFA devices are detected ([#141])
* Add the ability for corndog to generate the hugepages setting ([#141])
* Compile `host-ctr` with go 1.23 ([#146])
* Update `host-ctr` dependencies ([#146])
* Include `nvidia-cdi-hook` in `nvidia-container-toolkit` ([#150])
* Update kernels to 5.10.225 and 5.15.166 ([#154])
* Use Go 1.22 for kubernetes 1.23, 1.24, 1.25, 1.26, 1.27, 1.28, 1.29 ([#155])

[#141]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/141
[#146]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/146
[#150]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/150
[#153]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/153
[#154]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/154
[#155]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/155

# v2.6.0 (2024-09-17)

## Build Changes
* Validate `twoliter` upon install ([#147])

## OS Changes
* Add the ability for driverdog to copy modules ([#119])
* Add pciclient crate for high level access to `lspci` ([#149])
* Update 6.1 kernel to 6.1.109 ([#151])

[#119]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/119
[#147]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/147
[#149]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/149
[#151]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/151

# v2.5.0 (2024-09-11)

## Build Changes
* Update tough ([#136])
* Update bottlerocket-sdk to v0.45.0 ([#131])

## OS Changes
* Build open source NVIDIA kernel modules ([#118])
* Update third party packages ([#129], [#143])
* Split ECS and VPC CNI plugins from ecs-agent ([#85])
* Add helper functions for ipcidr ([#116])
* Add aws-otel-collector package ([#50])
* Add pciutils package ([#142])

## Orchestrator Changes

### Kubernetes
* Use kubelet-device-plugins API ([#132])

[#50]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/50
[#85]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/85
[#116]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/116
[#118]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/118
[#129]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/129
[#131]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/131
[#132]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/132
[#136]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/136
[#142]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/142
[#143]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/143

# v2.4.1 (2024-09-09)

## OS Changes
* Use direct paths for ephemeral storage ([#133])
* Update libexpat to 2.6.3 ([#130])

[#130]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/130
[#133]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/133

# v2.4.0 (2024-09-05)

## OS Changes
* Add ephemeral-storage commands ([#15]) - thanks @tzneal
* Add support for bootstrap commands ([#62], [#127])
* Update runc to 1.1.14 ([#123])
* Update kernels to 5.10.224, 5.15.165 and 6.1.106 ([#128], [#126])

## Orchestrator Changes

### Kubernetes
* Add Kubernetes 1.31 packages ([#117])
* Apply EKS-D upstream patches for Kubernetes v1.23 to v1.26 ([#121])
* Add latest instance types to eni-max-pods mapping ([#120])

[#15]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/15
[#62]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/62
[#117]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/117
[#120]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/120
[#121]:https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/121
[#123]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/123
[#126]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/126
[#127]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/127
[#128]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/128

# v2.3.6 (2024-08-28)

## Build Changes
* Update Twoliter to 0.4.5 (#106)
* schnauzer: add reflective template helpers (#105)
* Update bottlerocket-sdk to v0.44.0 ([#109])

## OS Changes
* Third party package updates (#108)

[#105]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/105
[#106]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/106
[#108]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/108
[#109]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/109

# v2.3.5 (2024-08-21)

## Orchestrator Changes

### Kubernetes

 * Fix issue where a null value would fail to render the credential
   provider template for Kubernetes ([#101])

## OS Changes

 * Improve EBS volume udev rules by adding a symlink to `/dev/by-ebs-id`
   and remove `/dev/` from the device name returned by ghostdog ([#98])
 * Update kernels to 5.10.223-212 and 6.1.102-111 ([#99])

## tools

 * Add collect-kernel-config script to tools ([#84])

[#84]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/84
[#98]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/98
[#99]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/99
[#101]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/101


# v2.3.4 (2024-08-19)
## OS Changes

* Update libnvidia-container to v550.54.14 and nvidia-container-toolkit to v1.16.1 ([#88])
* Fix a bug in sundog that caused it to regenerate populated settings ([#94])

[#88]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/88
[#94]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/94

# v2.3.3 (2024-08-14)

## Orchestrator Changes

### Kubernetes

* kubernetes 1.24, 1.25, 1.26: Apply upstream patches ([#76], [#77], [#78])
* packages: use `GO_MAJOR` for selecting Go version ([#86])

## Build Changes
* pluto: use settings SDK to parse API response ([#89])
* schnauzer: add support for update repository & ecr registry in ISO-E ([#91])

[#76]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/76
[#77]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/77
[#78]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/78
[#86]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/86
[#89]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/89
[#91]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/91

# v2.3.2 (2024-08-13)

## OS Changes

* Update kernels: 5.10.223 5.15.164, and 6.1.102 ([#73], [#82])

## Orchestrator Changes

### Kubernetes

* ecr-credential-provider: update to 1.25.15 ([#66])
* ecr-credential-provider-1.27: update to 1.27.8 ([#66])
* ecr-credential-provider-1.29: update to 1.29.6 ([#66])
* ecr-credential-provider-1.30: update to 1.30.3 ([#66])
* soci-snapshotter: update to 0.7.0 ([#66])

## Build Changes

* Re-enable fmt and licenses lints in CI ([#69])
* Use workspace dependencies for all dependencies ([#70])
* Update datastore serializer to expect JSON and correctly handle null values ([#80], [#87])

[#66]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/66
[#69]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/69
[#70]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/70
[#73]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/73
[#80]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/80
[#82]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/82
[#87]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/87

# v2.3.1 (2024-08-01)

## OS Changes

* Update docker-engine to v25.0.6 ([#55])

## Orchestrator Changes

### Kubernetes

* nvidia-container-runtime, nvidia-k8s-device-plugin: support Nvidia settings APIs [#48]
* Support hostname-override-source ([#59])

## Build Changes

* Update bottlerocket-settings-models to v0.2.0 ([#58])
* Update bottlerocket-sdk to v0.43.0 ([#60])

[#48]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/48
[#55]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/55
[#58]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/58
[#59]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/59
[#60]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/60

# v2.3.0 (2024-07-24)

## OS Changes

* Update containerd to 1.7.20 ([#40])
* Update runc to 1.1.13 ([#40])
* Update kernels: 5.10.220, 5.15.162, and 6.1.97 ([#46])
* Add kmod-6.1-neuron-devel ([#42])

## Orchestrator Changes

### Kubernetes

* Add latest instance types to eni-max-pods mapping ([#43])

## Build Changes

* Update Twoliter to 0.4.3 ([#39])

[#39]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/39
[#40]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/40
[#42]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/42
[#43]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/43
[#46]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/46

# v2.2.0 (2024-07-18)

## OS Changes
* Add libbpf ([#24], thanks @ndbaker1)
* Add kube-proxy ([#25], thanks @tzneal)
* Third party package updates ([#28], [#35], [#36])
* Update rust dependencies for first-party sources ([#34])
* Update kernels: 5.10.220, 5.15.161, and 6.1.96 ([#29])

## Build Changes
* Update `twoliter` ([#30], [#39])

## Tools
* Fix `diff-kernel-config` to work with Core Kit ([#32])

[#24]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/24
[#25]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/25
[#28]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/28
[#29]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/29
[#30]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/30
[#32]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/32
[#34]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/34
[#35]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/35
[#36]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/36
[#39]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/39

# v2.1.0 (2024-07-08)

## OS Changes
* Update kernels: 5.10.219, 5.15.160-104, and 6.1.94 ([#13], [#17])
* Add kmod-6.1-neuron package in core kit ([#21])
* Provide SSM agent as a system service ([#22])
* Enable host containers and in-place updates to be optional ([#23])

## Orchestrator Changes

### Kubernetes
* Move dockershim link to relative path ([#18])

[#13]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/13
[#17]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/17
[#18]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/18
[#21]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/21
[#22]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/22
[#23]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/23

# v2.0.0 (2024-06-20)

## Kit Features

* Move code to core kit ([#1])
* Use Bottlerocket Settings SDK for settings models ([#7])

## OS Changes

* Add mdadm packages for software RAID ([#4035]) - Thanks tzneal!
* Update kernels: 5.10.217, 5.15.156, and 6.1.92([#4049],[#4039], [#4005], [#3972], [#3976])
* Update containerd to 1.7.17 ([#4016])

## Build Changes

* Change pluto to act more like a settings generator ([#4032])
* Update pluto for kits and Out of Tree Builds ([#3828])
* Remove API Client dependency on the Settings model ([#3987])
* Create CloudFormation settings extension ([#4010])
* Add symlink to latest version for amazon-ssm-agent ([#3986])
* Prepare os package for build system changes ([#4006])
* Move to DNS settings extension ([#3980])
* Move to OCI Hooks Settings Extension ([#3978])
* Add Metrics Settings Extension ([#3963])
* Move to PKI Settings Extension ([#3971])
* Remove metadata migration ([#3958])
* Remove version from makefile ([#4])
* Improve cache behavior ([#6])
* Twoliter updates ([#8])

[#1]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/1
[#4]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/4
[#6]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/6
[#7]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/7
[#8]: https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/8
[#3828]: https://github.com/bottlerocket-os/bottlerocket/pull/3828
[#3958]: https://github.com/bottlerocket-os/bottlerocket/pull/3958
[#3963]: https://github.com/bottlerocket-os/bottlerocket/pull/3963
[#3971]: https://github.com/bottlerocket-os/bottlerocket/pull/3971
[#3972]: https://github.com/bottlerocket-os/bottlerocket/pull/3972
[#3976]: https://github.com/bottlerocket-os/bottlerocket/pull/3976
[#3978]: https://github.com/bottlerocket-os/bottlerocket/pull/3978
[#3980]: https://github.com/bottlerocket-os/bottlerocket/pull/3980
[#3987]: https://github.com/bottlerocket-os/bottlerocket/pull/3987
[#3986]: https://github.com/bottlerocket-os/bottlerocket/pull/3986
[#4005]: https://github.com/bottlerocket-os/bottlerocket/pull/4005
[#4006]: https://github.com/bottlerocket-os/bottlerocket/pull/4006
[#4010]: https://github.com/bottlerocket-os/bottlerocket/pull/4010
[#4016]: https://github.com/bottlerocket-os/bottlerocket/pull/4016
[#4032]: https://github.com/bottlerocket-os/bottlerocket/pull/4032
[#4035]: https://github.com/bottlerocket-os/bottlerocket/pull/4035
[#4039]: https://github.com/bottlerocket-os/bottlerocket/pull/4039
[#4049]: https://github.com/bottlerocket-os/bottlerocket/pull/4049
