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
    
[#39] https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/39
[#40] https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/40
[#42] https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/42
[#43] https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/43
[#46] https://github.com/bottlerocket-os/bottlerocket-core-kit/pull/46

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
