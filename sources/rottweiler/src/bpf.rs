use libbpf_rs::skel::{OpenSkel, Skel, SkelBuilder};
use snafu::prelude::*;
use std::path::Path;

mod skel {
    include!(concat!(env!("OUT_DIR"), "/bpf.skel.rs"));
}

type Result<T> = std::result::Result<T, snafu::Whatever>;

const BPF_PIN_PATH: &str = "/sys/fs/bpf/rottweiler";
const PROTECTED_MOUNTS_MAP: &str = "protected_devices";

pub fn open_protected_mounts_map() -> Result<libbpf_rs::MapHandle> {
    libbpf_rs::MapHandle::from_pinned_path(Path::new(BPF_PIN_PATH).join(PROTECTED_MOUNTS_MAP))
        .with_whatever_context(|_| "failed to open pinned protected_mounts map")
}

pub fn load_bpf() -> Result<libbpf_rs::MapHandle> {
    if Path::new(BPF_PIN_PATH).join("block_remount").exists()
        && Path::new(BPF_PIN_PATH).join("block_move_mount").exists()
        && Path::new(BPF_PIN_PATH).join("block_umount").exists()
    {
        return open_protected_mounts_map();
    }

    let skel_builder = skel::ProtectDirsSkelBuilder::default();
    let mut open_object = std::mem::MaybeUninit::uninit();

    let mut open_skel = skel_builder
        .open(&mut open_object)
        .with_whatever_context(|_| "failed to open BPF skeleton")?;

    open_skel
        .maps
        .protected_devices
        .set_pin_path(Path::new(BPF_PIN_PATH).join(PROTECTED_MOUNTS_MAP))
        .with_whatever_context(|_| "failed to set map pin path")?;

    let mut skel = open_skel
        .load()
        .with_whatever_context(|_| "failed to load BPF program")?;

    skel.attach()
        .with_whatever_context(|_| "failed to attach BPF program")?;

    std::fs::create_dir_all(BPF_PIN_PATH)
        .with_whatever_context(|_| "failed to create link directory")?;

    if let Some(ref mut link) = skel.links.block_remount {
        link.pin(Path::new(BPF_PIN_PATH).join("block_remount"))
            .with_whatever_context(|_| "failed to pin block_remount link")?;
    }

    if let Some(ref mut link) = skel.links.block_move_mount {
        link.pin(Path::new(BPF_PIN_PATH).join("block_move_mount"))
            .with_whatever_context(|_| "failed to pin block_move_mount link")?;
    }

    if let Some(ref mut link) = skel.links.block_umount {
        link.pin(Path::new(BPF_PIN_PATH).join("block_umount"))
            .with_whatever_context(|_| "failed to pin block_umount link")?;
    }

    open_protected_mounts_map()
}
