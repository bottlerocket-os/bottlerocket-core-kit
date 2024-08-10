# Bottlerocket Core Kit
This is the core kit for [Bottlerocket](https://github.com/bottlerocket-os/bottlerocket).
It includes many common dependencies for downstream package and variant builds.

## Contents
The core kit includes:
* Shared libraries such as glibc and libz
* Management daemons such as systemd and dbus-broker
* Agents for settings API and in-place updates

### Availability
The [Bottlerocket core kit](https://gallery.ecr.aws/bottlerocket/bottlerocket-core-kit) is available through Amazon ECR Public.

### Development
The core kit can be built on either an **x86_64** or an **aarch64** host. To do this you can use the following commands. 
```shell
make
```
OR
```shell
make ARCH=<aarch64, x86_64>
```
See the [BUILDING](BUILDING.md) guide for more details.
