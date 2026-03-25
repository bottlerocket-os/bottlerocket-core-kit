/* SPDX-License-Identifier: GPL-2.0 */
#ifndef __VMLINUX_MINIMAL_H__
#define __VMLINUX_MINIMAL_H__

typedef signed char __s8;
typedef unsigned char __u8;
typedef signed short __s16;
typedef unsigned short __u16;
typedef signed int __s32;
typedef unsigned int __u32;
typedef signed long long __s64;
typedef unsigned long long __u64;

typedef __u16 __be16;
typedef __u32 __be32;
typedef __u32 __wsum;

enum bpf_map_type {
    BPF_MAP_TYPE_HASH = 1,
};

typedef __u32 dev_t;

struct block_device {
    dev_t bd_dev;
} __attribute__((preserve_access_index));

struct super_block {
    dev_t s_dev;
    struct block_device *s_bdev;
} __attribute__((preserve_access_index));

struct vfsmount {
    struct super_block *mnt_sb;
} __attribute__((preserve_access_index));

struct path {
    struct vfsmount *mnt;
} __attribute__((preserve_access_index));

#endif /* __VMLINUX_MINIMAL_H__ */
