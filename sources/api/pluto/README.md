# pluto

Current version: 0.1.0

## Introduction

pluto is a special settings generator that runs only on Bottlerocket EKS nodes and is
responsible for generating settings required by Kubernetes. It is invoked through a
systemd service `pluto.service` after sundog generates rest of the settings but before
`settings-applier.service` commits them.

It uses IMDS to get information such as:

- Instance Type
- Node IP

It uses EKS to get information such as:

- Service IP CIDR

It uses the Bottlerocket API to get information such as:

- Kubernetes Cluster Name
- AWS Region

## Colophon

This text was generated using [cargo-readme](https://crates.io/crates/cargo-readme), and includes the rustdoc from `src/main.rs`.
