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
