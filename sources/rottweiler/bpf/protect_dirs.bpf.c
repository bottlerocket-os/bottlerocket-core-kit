// SPDX-License-Identifier: GPL-2.0
#include "vmlinux_minimal.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>
#include <bpf/bpf_core_read.h>

/* Constants not in vmlinux.h (preprocessor macros) */
#define EPERM 1

#define MAX_PROTECTED_DEVICES 1024

/* Map of protected device IDs */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __uint(max_entries, MAX_PROTECTED_DEVICES);
    __type(key, __u32);
    __type(value, __s32);
    __uint(pinning, LIBBPF_PIN_BY_NAME);
} protected_devices SEC(".maps");

/* Prevent remounting protected filesystems */
SEC("lsm/sb_remount")
int BPF_PROG(block_remount, struct super_block *sb, void *mnt_opts)
{
    __u32 dev = 0;
    __s32 *val = NULL;

    if (!sb)
        return 0;

    /* Get device ID */
    if (bpf_core_read(&dev, sizeof(dev), &sb->s_dev))
        return 0;

    /* Check if this device is protected */
    val = bpf_map_lookup_elem(&protected_devices, &dev);
    if (!val)
        return 0;

    bpf_printk("Blocked remount on protected device %u", dev);
    return -EPERM;
}

/* Prevent mounting over protected directories */
SEC("lsm/move_mount")
int BPF_PROG(block_move_mount, const struct path *from_path, const struct path *to_path)
{
    struct vfsmount *mnt = NULL;
    struct super_block *sb = NULL;
    __u32 dev = 0;
    __s32 *val = NULL;

    if (!to_path)
        return 0;

    /* Get the vfsmount of the target mount point */
    if (bpf_core_read(&mnt, sizeof(mnt), &to_path->mnt))
        return 0;
    if (!mnt)
        return 0;

    /* Get the superblock of the target mount point */
    if (bpf_core_read(&sb, sizeof(sb), &mnt->mnt_sb))
        return 0;
    if (!sb)
        return 0;

    /* Get device ID of the target mount point's filesystem */
    if (bpf_core_read(&dev, sizeof(dev), &sb->s_dev))
        return 0;

    /* Check if target is on a protected device */
    val = bpf_map_lookup_elem(&protected_devices, &dev);
    if (!val)
        return 0;

    bpf_printk("Blocked mount over protected device %u", dev);
    return -EPERM;
}

/* Prevent unmounting protected filesystems */
SEC("lsm/sb_umount")
int BPF_PROG(block_umount, struct vfsmount *mnt, int flags)
{
    struct super_block *sb = NULL;
    __u32 dev = 0;
    __s32 *val = NULL;

    if (!mnt)
        return 0;

    /* Get the superblock */
    if (bpf_core_read(&sb, sizeof(sb), &mnt->mnt_sb))
        return 0;
    if (!sb)
        return 0;

    /* Get device ID */
    if (bpf_core_read(&dev, sizeof(dev), &sb->s_dev))
        return 0;

    /* Check if this device is protected */
    val = bpf_map_lookup_elem(&protected_devices, &dev);
    if (!val)
        return 0;

    bpf_printk("Blocked umount on protected device %u", dev);
    return -EPERM;
}

char _license[] SEC("license") = "GPL";
