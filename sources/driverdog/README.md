# driverdog

Current version: 0.1.0

driverdog is a tool to link kernel modules at runtime. It uses a toml configuration file with the following shape:

`lib-modules-path`: destination path for the .ko linked files
`objects-source`: path where the objects used to link the kernel module are
`object-files`: hash with the object files to be linked, each object in the map should include the files used to link it
`kernel-modules`: hash with the kernel modules to be linked, each kernel module in the map should include the files used to link it

There are two modes for driverdog: link then load and copy then load. Link then load takes unlinked files found in `objects-source`
and matched in `object-files` and `kernel-modules` to link together these files then copy them to `lib-modules-path`. Copy then load
finds the modules specified in `kernel-modules` and copies them to `lib-modules-path` from the source specified in `copy-source`. Both
modes iterate over the `kernel-modules` and load them from that path with `modprobe`.

## Colophon

This text was generated from `README.tpl` using [cargo-readme](https://crates.io/crates/cargo-readme), and includes the rustdoc from `src/main.rs`.
