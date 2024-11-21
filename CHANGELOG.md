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
