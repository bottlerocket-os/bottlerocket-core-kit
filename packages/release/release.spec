%global debug_package %{nil}
%global _cross_first_party 1

Name: %{_cross_os}release
Version: 0.0
Release: 1%{?dist}
Epoch: 1
Summary: Bottlerocket release
License: Apache-2.0 OR MIT
URL: https://github.com/bottlerocket-os/bottlerocket

# Core config files.
Source11: nsswitch.conf
Source12: issue

# Sysctl drop-ins.
Source80: release-sysctl.conf
Source81: release-swap-sysctl.conf

# Other drop-ins.
Source93: release-tmpfiles.conf
Source94: release-fips-tmpfiles.conf
Source95: release-systemd-networkd.conf
Source96: release-repart-local.conf
Source98: release-systemd-system.conf
Source99: release-ca-certificates-tmpfiles.conf
Source100: release-modules-load.conf
Source101: release-systemd-journald.conf

# Templates for the settings API.
Source200: motd.template
Source201: proxy-env
Source202: hostname-env
Source203: hosts.template
Source204: modprobe-conf.template
Source205: netdog.template
Source206: aws-config
Source207: aws-credentials
Source208: modules-load.template
Source209: log4j-hotpatch-enabled
Source210: selected-snapshotter-template

# Core targets, services, and slices.
Source1001: multi-user.target
Source1002: configured.target
Source1003: preconfigured.target
Source1004: fipscheck.target
Source1005: activate-preconfigured.service
Source1006: activate-configured.service
Source1007: activate-multi-user.service
Source1008: set-hostname.service
Source1009: runtime.slice
Source1010: drivers.target

# Mount units.
Source1020: var.mount
Source1021: opt.mount
Source1022: var-lib-bottlerocket.mount
Source1023: etc-cni.mount
Source1024: mnt.mount
Source1025: local.mount
Source1026: media-cdrom.mount
Source1027: root-.aws.mount
Source1028: opt-cni.mount
Source1029: opt-csi.mount

# Mounts that require helper programs.
Source1040: prepare-boot.service
Source1041: prepare-opt.service
Source1042: prepare-var.service
Source1043: repart-local.service
Source1044: mask-local-mnt.service
Source1045: mask-local-opt.service
Source1046: mask-local-var.service
Source1047: repart-data-preferred.service
Source1048: repart-data-fallback.service
Source1049: prepare-local-fs.service
Source1050: prepare-swap.service

# Feature-specific units.
Source1060: capture-kernel-dump.service
Source1061: disable-kexec-load.service
Source1062: load-crash-kernel.service
Source1063: deprecation-warning@.service
Source1064: deprecation-warning@.timer
Source1065: check-kernel-integrity.service
Source1066: check-fips-modules.service
Source1067: fips-modprobe@.service
Source1068: configure-snapshotter.service

# Mounts that require build-time edits.
Source1080: var-lib-kernel-devel-lower.mount.in
Source1081: usr-src-kernels.mount.in
Source1082: usr-share-licenses.mount.in
Source1083: lib-modules.mount.in
Source1084: usr-bin.mount.in
Source1085: usr-libexec.mount.in
Source1086: bottlerocket.mount.in

# Drop-in units to override defaults
Source1100: systemd-tmpfiles-setup-service-debug.conf
Source1101: systemd-resolved-service-env.conf
Source1102: systemd-networkd-service-env.conf
Source1103: systemd-logind-inhibit-maxdelay.conf
Source1104: aws-config.conf
Source1105: wait-for-selinux-policy.conf
Source1106: systemd-resolved-service-private-tmp.conf
Source1107: systemd-journald-compat.conf
Source1108: systemd-sysusers-selinux.conf
Source1109: modprobe-no-exit.conf
Source1110: tmp-mount-noexec.conf

# network link rules
Source1200: 80-release.link

# udev rules
Source1300: mount-cdrom.rules

# Common logdog configuration
Source1400: logdog.common.conf

# bootconfig snippets.
Source1500: bootconfig-fips.conf

Requires: %{_cross_os}audit
Requires: %{_cross_os}chrony
Requires: %{_cross_os}conntrack-tools
Requires: %{_cross_os}containerd
Requires: %{_cross_os}coreutils
Requires: %{_cross_os}dbus-broker
Requires: %{_cross_os}e2fsprogs
Requires: %{_cross_os}early-boot-config
Requires: %{_cross_os}ethtool
Requires: %{_cross_os}libgcc
Requires: %{_cross_os}libstd-rust
Requires: %{_cross_os}filesystem
Requires: %{_cross_os}findutils
Requires: %{_cross_os}glibc
Requires: %{_cross_os}grep
Requires: %{_cross_os}grub
Requires: %{_cross_os}iproute
Requires: %{_cross_os}iptables
Requires: %{_cross_os}kexec-tools
Requires: %{_cross_os}keyutils
Requires: %{_cross_os}makedumpfile
Requires: %{_cross_os}mdadm
Requires: %{_cross_os}netdog
Requires: %{_cross_os}os
Requires: %{_cross_os}pciutils
Requires: %{_cross_os}policycoreutils
Requires: %{_cross_os}procps
Requires: (%{_cross_os}rdma-core if %{_cross_os}variant-platform(aws))
Requires: %{_cross_os}selinux-policy
Requires: %{_cross_os}shim
Requires: %{_cross_os}systemd
Requires: %{_cross_os}util-linux
Requires: %{_cross_os}xfsprogs
Requires: (%{name}-fips if %{_cross_os}image-feature(fips))

%description
%{summary}.

%package fips
Summary: Bottlerocket release, FIPS edition
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: %{_cross_os}image-feature(no-fips)
Requires: %{_cross_os}libkcapi

%description fips
%{summary}.

%package swap
Summary: Bottlerocket release, with zram-based swap
Requires: %{name}
Requires: (%{name}-fips if %{_cross_os}image-feature(fips))

%description swap
%{summary}.

%prep

%build

%install
install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}
install -p -m 0644 %{S:11} %{S:12} %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}

install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m 0644 %{S:93} %{buildroot}%{_cross_tmpfilesdir}/release.conf
install -p -m 0644 %{S:99} %{buildroot}%{_cross_tmpfilesdir}/release-ca-certificates.conf
install -p -m 0644 %{S:94} %{buildroot}%{_cross_tmpfilesdir}/release-fips.conf

install -d %{buildroot}%{_cross_libdir}/systemd/networkd.conf.d
install -p -m 0644 %{S:95} %{buildroot}%{_cross_libdir}/systemd/networkd.conf.d/80-release.conf

install -d %{buildroot}%{_cross_libdir}/repart.d/
install -p -m 0644 %{S:96} %{buildroot}%{_cross_libdir}/repart.d/80-local.conf

install -d %{buildroot}%{_cross_sysctldir}
install -p -m 0644 %{S:80} %{buildroot}%{_cross_sysctldir}/80-release.conf
install -p -m 0644 %{S:81} %{buildroot}%{_cross_sysctldir}/81-release-swap.conf

install -d %{buildroot}%{_cross_unitdir}/service.d
install -p -m 0644 %{S:1104} %{buildroot}%{_cross_unitdir}/service.d/00-aws-config.conf

install -d %{buildroot}%{_cross_libdir}/systemd/system.conf.d
install -p -m 0644 %{S:98} %{buildroot}%{_cross_libdir}/systemd/system.conf.d/80-release.conf

install -d %{buildroot}%{_cross_libdir}/modules-load.d
install -p -m 0644 %{S:100} %{buildroot}%{_cross_libdir}/modules-load.d/nf_conntrack.conf

install -d %{buildroot}%{_cross_libdir}/systemd/journald.conf.d
install -p -m 0644 %{S:101} %{buildroot}%{_cross_libdir}/systemd/journald.conf.d/journald.conf

install -d %{buildroot}%{_cross_libdir}/systemd/network
install -p -m 0644 %{S:1200} %{buildroot}%{_cross_libdir}/systemd/network/80-release.link

install -d %{buildroot}%{_cross_libdir}/systemd/logind.conf.d
install -p -m 0644 %{S:1103} %{buildroot}%{_cross_libdir}/systemd/logind.conf.d/80-inhibit-maxdelay.conf

cat >%{buildroot}%{_cross_libdir}/os-release <<EOF
NAME=Bottlerocket
ID=bottlerocket
EOF

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 \
  %{S:1001} %{S:1002} %{S:1003} %{S:1004} %{S:1005} \
  %{S:1006} %{S:1007} %{S:1008} %{S:1009} %{S:1010} \
  %{S:1020} %{S:1021} %{S:1022} %{S:1023} %{S:1024} \
  %{S:1025} %{S:1026} %{S:1027} %{S:1028} %{S:1029} \
  %{S:1040} %{S:1041} %{S:1042} %{S:1043} %{S:1044} \
  %{S:1045} %{S:1046} %{S:1047} %{S:1048} %{S:1049} \
  %{S:1050} \
  %{S:1060} %{S:1061} %{S:1062} %{S:1063} %{S:1064} \
  %{S:1065} %{S:1066} %{S:1067} %{S:1068} \
  %{buildroot}%{_cross_unitdir}

install -d %{buildroot}%{_cross_unitdir}/systemd-tmpfiles-setup.service.d
install -p -m 0644 %{S:1100} \
  %{buildroot}%{_cross_unitdir}/systemd-tmpfiles-setup.service.d/00-debug.conf

install -d %{buildroot}%{_cross_unitdir}/systemd-resolved.service.d
install -p -m 0644 %{S:1101} \
  %{buildroot}%{_cross_unitdir}/systemd-resolved.service.d/00-env.conf
install -p -m 0644 %{S:1106} \
  %{buildroot}%{_cross_unitdir}/systemd-resolved.service.d/10-private-tmp.conf

install -d %{buildroot}%{_cross_unitdir}/systemd-networkd.service.d
install -p -m 0644 %{S:1102} \
  %{buildroot}%{_cross_unitdir}/systemd-networkd.service.d/00-env.conf

install -d %{buildroot}%{_cross_unitdir}/systemd-udev-trigger.service.d/
install -p -m 0644 %{S:1105} \
  %{buildroot}%{_cross_unitdir}/systemd-udev-trigger.service.d/00-selinux.conf

install -d %{buildroot}%{_cross_unitdir}/systemd-journald.service.d
install -p -m 0644 %{S:1107} \
  %{buildroot}%{_cross_unitdir}/systemd-journald.service.d/50-systemd-journald.conf

install -d %{buildroot}%{_cross_unitdir}/systemd-sysusers.service.d
install -p -m 0644 %{S:1108} \
  %{buildroot}%{_cross_unitdir}/systemd-sysusers.service.d/50-systemd-sysusers.conf

install -d %{buildroot}%{_cross_unitdir}/modprobe@.service.d
install -p -m 0644 %{S:1109} \
  %{buildroot}%{_cross_unitdir}/modprobe@.service.d/10-remain-after-exit.conf

install -d %{buildroot}%{_cross_unitdir}/tmp.mount.d
install -p -m 0644 %{S:1110} \
  %{buildroot}%{_cross_unitdir}/tmp.mount.d/10-no-exec.conf

# Empty (but packaged) directory. The FIPS packages for kernels will add drop-ins to
# this directory to arrange for the right modules to be loaded before the check runs.
install -d %{buildroot}%{_cross_unitdir}/check-fips-modules.service.d

LOWERPATH=$(systemd-escape --path %{_cross_sharedstatedir}/kernel-devel/.overlay/lower)
sed -e 's|PREFIX|%{_cross_prefix}|g' %{S:1080} > ${LOWERPATH}.mount
install -p -m 0644 ${LOWERPATH}.mount %{buildroot}%{_cross_unitdir}

# Mounting on usr/src/kernels requires using the real path: %{_cross_usrsrc}/kernels
KERNELPATH=$(systemd-escape --path %{_cross_usrsrc}/kernels)
sed -e 's|PREFIX|%{_cross_prefix}|g' %{S:1081} > ${KERNELPATH}.mount
install -p -m 0644 ${KERNELPATH}.mount %{buildroot}%{_cross_unitdir}

# Mounting on usr/share/licenses requires using the real path: %{_cross_datadir}/licenses
LICENSEPATH=$(systemd-escape --path %{_cross_licensedir})
sed -e 's|PREFIX|%{_cross_prefix}|g' %{S:1082} > ${LICENSEPATH}.mount
install -p -m 0644 ${LICENSEPATH}.mount %{buildroot}%{_cross_unitdir}

# Mounting on lib/modules requires using the real path: %{_cross_libdir}/modules
LIBDIRPATH=$(systemd-escape --path %{_cross_libdir})
sed -e 's|PREFIX|%{_cross_prefix}|g' %{S:1083} > ${LIBDIRPATH}-modules.mount
install -p -m 0644 ${LIBDIRPATH}-modules.mount %{buildroot}%{_cross_unitdir}

# Mounting on usr/bin requires using the real path: %{_cross_bindir}
BINDIRPATH=$(systemd-escape --path %{_cross_bindir})
sed -e 's|PREFIX|%{_cross_prefix}|g' %{S:1084} > ${BINDIRPATH}.mount
install -p -m 0644 ${BINDIRPATH}.mount %{buildroot}%{_cross_unitdir}

# Mounting on usr/libexec requires using the real path: %{_cross_libexecdir}
LIBEXECDIRPATH=$(systemd-escape --path %{_cross_libexecdir})
sed -e 's|PREFIX|%{_cross_prefix}|g' %{S:1085} > ${LIBEXECDIRPATH}.mount
install -p -m 0644 ${LIBEXECDIRPATH}.mount %{buildroot}%{_cross_unitdir}

# Process bottlerocket mount template files with proper systemd naming
BOTTLEROCKET_PATH=$(systemd-escape --path /.bottlerocket)
install -p -m 0644 %{S:1086} %{buildroot}%{_cross_unitdir}/${BOTTLEROCKET_PATH}.mount

install -d %{buildroot}%{_cross_templatedir}
install -p -m 0644 %{S:200} %{buildroot}%{_cross_templatedir}/motd
install -p -m 0644 %{S:201} %{buildroot}%{_cross_templatedir}/proxy-env
install -p -m 0644 %{S:202} %{buildroot}%{_cross_templatedir}/hostname-env
install -p -m 0644 %{S:203} %{buildroot}%{_cross_templatedir}/hosts
install -p -m 0644 %{S:204} %{buildroot}%{_cross_templatedir}/modprobe-conf
install -p -m 0644 %{S:205} %{buildroot}%{_cross_templatedir}/netdog-toml
install -p -m 0644 %{S:206} %{buildroot}%{_cross_templatedir}/aws-config
install -p -m 0644 %{S:207} %{buildroot}%{_cross_templatedir}/aws-credentials
install -p -m 0644 %{S:208} %{buildroot}%{_cross_templatedir}/modules-load
install -p -m 0644 %{S:209} %{buildroot}%{_cross_templatedir}/log4j-hotpatch-enabled
install -p -m 0644 %{S:210} %{buildroot}%{_cross_templatedir}/selected-snapshotter

install -d %{buildroot}%{_cross_udevrulesdir}
install -p -m 0644 %{S:1300} %{buildroot}%{_cross_udevrulesdir}/61-mount-cdrom.rules

install -d %{buildroot}%{_cross_datadir}/logdog.d
install -p -m 0644 %{S:1400} %{buildroot}%{_cross_datadir}/logdog.d

install -d %{buildroot}%{_cross_bootconfigdir}
install -p -m 0644 %{S:1500} %{buildroot}%{_cross_bootconfigdir}/10-fips.conf

ln -s preconfigured.target %{buildroot}%{_cross_unitdir}/default.target

%files
%{_cross_factorydir}%{_cross_sysconfdir}/nsswitch.conf
%{_cross_factorydir}%{_cross_sysconfdir}/issue
%{_cross_sysctldir}/80-release.conf
%{_cross_tmpfilesdir}/release.conf
%{_cross_tmpfilesdir}/release-ca-certificates.conf
%{_cross_libdir}/os-release
%dir %{_cross_libdir}/repart.d
%{_cross_libdir}/repart.d/80-local.conf
%{_cross_libdir}/systemd/logind.conf.d/80-inhibit-maxdelay.conf
%{_cross_libdir}/systemd/network/80-release.link
%{_cross_libdir}/systemd/networkd.conf.d/80-release.conf
%{_cross_libdir}/systemd/system.conf.d/80-release.conf
%{_cross_libdir}/modules-load.d/nf_conntrack.conf
%{_cross_libdir}/systemd/journald.conf.d/journald.conf
%{_cross_unitdir}/configured.target
%{_cross_unitdir}/drivers.target
%{_cross_unitdir}/preconfigured.target
%{_cross_unitdir}/multi-user.target
%{_cross_unitdir}/default.target
%{_cross_unitdir}/activate-configured.service
%{_cross_unitdir}/activate-multi-user.service
%{_cross_unitdir}/disable-kexec-load.service
%{_cross_unitdir}/capture-kernel-dump.service
%{_cross_unitdir}/load-crash-kernel.service
%{_cross_unitdir}/prepare-boot.service
%{_cross_unitdir}/prepare-opt.service
%{_cross_unitdir}/prepare-var.service
%{_cross_unitdir}/repart-local.service
%{_cross_unitdir}/\x2ebottlerocket.mount
%{_cross_unitdir}/var.mount
%{_cross_unitdir}/opt.mount
%{_cross_unitdir}/mnt.mount
%{_cross_unitdir}/etc-cni.mount
%{_cross_unitdir}/opt-cni.mount
%{_cross_unitdir}/opt-csi.mount
%{_cross_unitdir}/media-cdrom.mount
%{_cross_unitdir}/local.mount
%{_cross_unitdir}/*-lower.mount
%{_cross_unitdir}/*-kernels.mount
%{_cross_unitdir}/*-licenses.mount
%{_cross_unitdir}/var-lib-bottlerocket.mount
%{_cross_unitdir}/*-modules.mount
%{_cross_unitdir}/runtime.slice
%{_cross_unitdir}/set-hostname.service
%{_cross_unitdir}/mask-local-mnt.service
%{_cross_unitdir}/mask-local-opt.service
%{_cross_unitdir}/mask-local-var.service
%{_cross_unitdir}/root-.aws.mount
%{_cross_unitdir}/repart-data-preferred.service
%{_cross_unitdir}/repart-data-fallback.service
%{_cross_unitdir}/prepare-local-fs.service
%{_cross_unitdir}/deprecation-warning@.service
%{_cross_unitdir}/deprecation-warning@.timer
%{_cross_unitdir}/service.d/00-aws-config.conf
%dir %{_cross_unitdir}/systemd-resolved.service.d
%{_cross_unitdir}/systemd-resolved.service.d/00-env.conf
%{_cross_unitdir}/systemd-resolved.service.d/10-private-tmp.conf
%dir %{_cross_unitdir}/systemd-networkd.service.d
%{_cross_unitdir}/systemd-networkd.service.d/00-env.conf
%dir %{_cross_unitdir}/systemd-tmpfiles-setup.service.d
%{_cross_unitdir}/systemd-tmpfiles-setup.service.d/00-debug.conf
%dir %{_cross_unitdir}/systemd-udev-trigger.service.d
%{_cross_unitdir}/systemd-udev-trigger.service.d/00-selinux.conf
%dir %{_cross_unitdir}/systemd-sysusers.service.d
%{_cross_unitdir}/systemd-sysusers.service.d/50-systemd-sysusers.conf
%dir %{_cross_unitdir}/systemd-journald.service.d
%{_cross_unitdir}/systemd-journald.service.d/50-systemd-journald.conf
%dir %{_cross_unitdir}/modprobe@.service.d
%{_cross_unitdir}/modprobe@.service.d/10-remain-after-exit.conf
%dir %{_cross_unitdir}/tmp.mount.d
%{_cross_unitdir}/tmp.mount.d/10-no-exec.conf
%dir %{_cross_templatedir}
%{_cross_templatedir}/modprobe-conf
%{_cross_templatedir}/netdog-toml
%{_cross_templatedir}/motd
%{_cross_templatedir}/proxy-env
%{_cross_templatedir}/hostname-env
%{_cross_templatedir}/hosts
%{_cross_templatedir}/aws-config
%{_cross_templatedir}/aws-credentials
%{_cross_templatedir}/modules-load
%{_cross_templatedir}/log4j-hotpatch-enabled
%{_cross_udevrulesdir}/61-mount-cdrom.rules
%{_cross_datadir}/logdog.d/logdog.common.conf
%{_cross_unitdir}/configure-snapshotter.service
%{_cross_templatedir}/selected-snapshotter

%files fips
%{_cross_bootconfigdir}/10-fips.conf
%{_cross_tmpfilesdir}/release-fips.conf
%{_cross_unitdir}/*-bin.mount
%{_cross_unitdir}/*-libexec.mount
%{_cross_unitdir}/fipscheck.target
%{_cross_unitdir}/activate-preconfigured.service
%{_cross_unitdir}/check-kernel-integrity.service
%{_cross_unitdir}/check-fips-modules.service
%dir %{_cross_unitdir}/check-fips-modules.service.d
%{_cross_unitdir}/fips-modprobe@.service

%files swap
%{_cross_sysctldir}/81-release-swap.conf
%{_cross_unitdir}/prepare-swap.service

%changelog
