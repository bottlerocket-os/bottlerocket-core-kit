use libbpf_cargo::SkeletonBuilder;
use std::env;
use std::path::PathBuf;

fn main() {
    generate_readme::from_main().unwrap();

    println!("cargo:rerun-if-changed=bpf/protect_dirs.bpf.c");
    println!("cargo:rerun-if-changed=bpf/vmlinux_minimal.h");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    SkeletonBuilder::new()
        .source("bpf/protect_dirs.bpf.c")
        .build_and_generate(out_dir.join("bpf.skel.rs"))
        .unwrap();
}
