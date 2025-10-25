# Skip check-rpaths since we expect them for systemd.
%global __brp_check_rpaths %{nil}

%global package_priority_epoch 1

Name: %{_cross_os}systemd-252
Version: 252.22
Release: 1%{?dist}
Summary: System and Service Manager
License: GPL-2.0-or-later AND GPL-2.0-only AND LGPL-2.1-or-later
URL: https://www.freedesktop.org/wiki/Software/systemd
Source0: https://github.com/systemd/systemd-stable/archive/v%{version}/systemd-stable-%{version}.tar.gz
Source100: org.freedesktop.login1.toml
Source101: org.freedesktop.network1.toml
Source102: org.freedesktop.resolve1.toml
Source103: org.freedesktop.systemd1.toml

# Backport of upstream patches that make the netlink default timeout
# configurable.  Bottlerocket carries this patch and configures the timeout in
# an effort to avoid a situation where a network link becomes unusable if the
# system is under load and doesn't process the RTM_NEWROUTE acknowledgement
# within the default timeout of 25 seconds.
# Reference issue: github.com/systemd/systemd/issues/25441
Patch1001: 1001-sd-netlink-make-calc_elapse-return-USEC_INFINITY-whe.patch
Patch1002: 1002-sd-netlink-make-the-default-timeout-configurable-by-.patch

# Backport of upstream patch to speed up `systemctl daemon-reload`.
Patch1003: 1003-serialize-don-t-allocate-1M-on-the-stack-just-like-t.patch

# Local patch to work around the fact that /var is a bind mount from
# /local/var, and we want the /local/var/run symlink to point to /run.
Patch9001: 9001-use-absolute-path-for-var-run-symlink.patch

# TODO: this could potentially be submitted upstream, but needs a better
# way to be configured at build time or during execution first.
Patch9002: 9002-core-add-separate-timeout-for-system-shutdown.patch

# TODO: this could be submitted upstream as well, but needs to account for
# the dom0 case first, where the UUID is all zeroes and hence not unique.
Patch9003: 9003-machine-id-setup-generate-stable-ID-under-Xen-and-VM.patch

# Local patch to mount /tmp with "noexec".
Patch9004: 9004-units-mount-tmp-with-noexec.patch

# Local patch to mount additional filesystems with "noexec".
Patch9005: 9005-mount-setup-apply-noexec-to-more-mounts.patch

# Local patch to handle mounting /etc with our SELinux label.
Patch9006: 9006-mount-setup-mount-etc-with-specific-label.patch

# We need `prefix` to be configurable for our own packaging so we can avoid
# dependencies on the host OS.
Patch9007: 9007-pkg-config-stop-hardcoding-prefix-to-usr.patch

# Local patch to stop overriding rp_filter defaults with wildcard values.
Patch9008: 9008-sysctl-do-not-set-rp_filter-via-wildcard.patch

# Local patch to set root's shell to /sbin/nologin rather than /bin/sh.
Patch9009: 9009-sysusers-set-root-shell-to-sbin-nologin.patch

# Local patch to keep modprobe units running to avoid repeated log entries.
Patch9010: 9010-units-keep-modprobe-service-units-running.patch

# Local patch to conditionalize systemd-networkd calls to hostname and timezone
# DBUS services not used in Bottlerocket
Patch9011: 9011-systemd-networkd-Conditionalize-hostnamed-timezoned-.patch

# Local patch to adjust the default mount rate limit to 25 per second.
# Carried as a patch so that SYSTEMD_DEFAULT_MOUNT_RATE_LIMIT_BURST can be used
# as a kernel command line parameter to override.
Patch9012: 9012-core-mount-increase-mount-rate-limit-burst-to-25.patch

# Local patch to work around a potentially non-compliant Option 15 in the DHCP
# lease in EC2.
Patch9013: 9013-sd-dhcp-lease-parse-multiple-domains-in-option-15.patch

# Local patch that allows to deselect systemd-gpt-auto-generator. We deselect
# it since prairiedog mounts /boot depending on the partition bank in use.
Patch9014: 9014-meson-make-gpt-auto-generator-selectable-at-build-ti.patch

# Local patch to allow resolving .local domains
Patch9015: 9015-allow-lookups-of-local-domains-using-unicast-DNS.patch

# Do not enable OpenSSL for systemd-resolved, to keep DNSSEC disabled.
Patch9016: 9016-meson-always-set-HAVE_OPENSSL_OR_GCRYPT-to-false.patch

# Do not enable OpenSSL for systemd-dissect, since AWS-LC doesn't support the
# PKCS7_verify function it wants.
Patch9017: 9017-dissect-image-disable-openssl-support.patch

# Have pkgconfig find "libcrypto.pc" instead of "openssl.pc" to avoid the
# unneeded dependency on libssl.
Patch9018: 9018-meson-replace-openssl-dependency-with-libcrypto.patch

# Downgrade warning logs for unit files with permission not set to 0044 since
# it does not apply to Bottlerocket where the API is restricted by the SELinux
# policy
Patch9019: 9019-suppress-log-for-units-with-mode-0044.patch

BuildRequires: gperf
BuildRequires: intltool
BuildRequires: meson
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}kmod-devel
BuildRequires: %{_cross_os}libacl-devel
BuildRequires: %{_cross_os}libattr-devel
BuildRequires: %{_cross_os}libblkid-devel
BuildRequires: %{_cross_os}libcap-devel
BuildRequires: %{_cross_os}libcrypto-devel
BuildRequires: %{_cross_os}libcryptsetup-devel
BuildRequires: %{_cross_os}libfdisk-devel
BuildRequires: %{_cross_os}libmount-devel
BuildRequires: %{_cross_os}libseccomp-devel
BuildRequires: %{_cross_os}libselinux-devel
BuildRequires: %{_cross_os}libtss2-devel
BuildRequires: %{_cross_os}libuuid-devel
BuildRequires: %{_cross_os}libxcrypt-devel

Requires: %{_cross_os}kmod
Requires: %{_cross_os}libacl
Requires: %{_cross_os}libattr
Requires: %{_cross_os}libblkid
Requires: %{_cross_os}libcap
Requires: %{_cross_os}libcrypto
Requires: %{_cross_os}libcryptsetup
Requires: %{_cross_os}libfdisk
Requires: %{_cross_os}libmount
Requires: %{_cross_os}libseccomp
Requires: %{_cross_os}libselinux
Requires: %{_cross_os}libtss2
Requires: %{_cross_os}libuuid
Requires: %{_cross_os}libxcrypt

Provides: %{_cross_os}systemd = %{package_priority_epoch}:
Provides: %{_cross_os}libudev = %{package_priority_epoch}:
Conflicts: %{_cross_os}systemd

%description
%{summary}.

%package console
Summary: Files for console login using the System and Service Manager
Requires: %{name}
Provides: %{_cross_os}systemd-console = %{package_priority_epoch}:

%description console
%{summary}.

%package cryptsetup
Summary: Files for cryptsetup support in systemd
Requires: %{name}
Requires: %{_cross_os}cryptsetup
Provides: %{_cross_os}systemd-cryptsetup = %{package_priority_epoch}:

%description cryptsetup
%{summary}.

%package devel
Summary: Files for development using the System and Service Manager
Requires: %{name}
Provides: %{_cross_os}systemd-devel = %{package_priority_epoch}:
Provides: %{_cross_os}libudev-devel = %{package_priority_epoch}:

%description devel
%{summary}.

%package networkd
Summary: Files for networkd
Requires: %{name}
Provides: %{_cross_os}systemd-networkd = %{package_priority_epoch}:

%description networkd
%{summary}.

%package resolved
Summary: Files for resolved
Requires: %{name}
Provides: %{_cross_os}systemd-resolved = %{package_priority_epoch}:

%description resolved
%{summary}.

%prep
%autosetup -n systemd-stable-%{version} -p1

%build
CONFIGURE_OPTS=(
 -Dmode=release

 -Dsplit-usr=false
 -Dsplit-bin=true
 -Drootprefix='%{_cross_prefix}'
 -Drootlibdir='%{_cross_libdir}'
 -Dlink-udev-shared=true
 -Dlink-systemctl-shared=true
 -Dlink-networkd-shared=false
 -Dlink-timesyncd-shared=false
 -Dlink-boot-shared=false
 -Dstatic-libsystemd=false
 -Dstatic-libudev=false

 -Dsysvinit-path=''
 -Dsysvrcnd-path=''
 -Dinitrd=false
 -Dnscd=false

 -Dutmp=false
 -Dhibernate=false
 -Dldconfig=true
 -Dresolve=true
 -Defi=true
 -Dtpm=true
 -Denvironment-d=false
 -Dbinfmt=true
 -Drepart=true
 -Dcoredump=false
 -Dpstore=true
 -Doomd=false
 -Dlogind=true
 -Dhostnamed=false
 -Dlocaled=false
 -Dmachined=false
 -Dportabled=false
 -Dsysext=false
 -Dsysupdate=false
 -Duserdb=false
 -Dhomed=false
 -Dnetworkd=true
 -Dtimedated=false
 -Dtimesyncd=false
 -Dremote=false
 -Dnss-myhostname=false
 -Dnss-mymachines=false
 -Dnss-resolve=true
 -Dnss-systemd=false
 -Dfirstboot=false
 -Drandomseed=true
 -Dbacklight=false
 -Dvconsole=false
 -Dquotacheck=false
 -Dsysusers=true
 -Dtmpfiles=true
 -Dimportd=false
 -Dhwdb=false
 -Drfkill=false
 -Dxdg-autostart=false
 -Dman=false
 -Dhtml=false
 -Dtranslations=false
 -Dgpt-auto-generator=false
 -Dlog-message-verification=false

 -Dcertificate-root='%{_cross_sysconfdir}/ssl'
 -Dpkgconfigdatadir='%{_cross_pkgconfigdir}'
 -Dpkgconfiglibdir='%{_cross_pkgconfigdir}'

 -Ddefault-hierarchy=unified

 -Dadm-group=false
 -Dwheel-group=false

 -Dgshadow=true

 -Ddefault-dnssec=no
 -Ddefault-dns-over-tls=no
 -Ddefault-mdns=no
 -Ddefault-llmnr=no
 -Ddns-over-tls=false
 -Ddns-servers=""

 -Dsupport-url="https://github.com/bottlerocket-os/bottlerocket/discussions"

 -Dseccomp=auto
 -Dselinux=auto
 -Dapparmor=false
 -Dsmack=false
 -Dpolkit=false
 -Dima=false

 -Dacl=true
 -Daudit=false
 -Dblkid=true
 -Dfdisk=true
 -Dkmod=true
 -Dpam=false
 -Dpwquality=false
 -Dmicrohttpd=false
 -Dlibcryptsetup=true
 -Dlibcryptsetup-plugins=true
 -Dlibcurl=false
 -Didn=false
 -Dlibidn2=false
 -Dlibidn=false
 -Dlibiptc=false
 -Dqrencode=false
 -Dgcrypt=false
 -Dgnutls=false
 -Dopenssl=true
 -Dp11kit=false
 -Dlibfido2=false
 -Dtpm2=true
 -Delfutils=false
 -Dzlib=false
 -Dbzip2=false
 -Dxz=false
 -Dlz4=false
 -Dzstd=false
 -Dxkbcommon=false
 -Dpcre2=false
 -Dglib=false
 -Ddbus=false

 -Dgnu-efi=false
 -Defi-tpm-pcr-compat=false

 -Dbashcompletiondir=no
 -Dzshcompletiondir=no

 -Dtests=false
 -Dslow-tests=false
 -Dfuzz-tests=false
 -Dinstall-tests=false

 -Durlify=false
 -Dfexecve=false

 -Doss-fuzz=false
 -Dllvm-fuzz=false
 -Dkernel-install=false
 -Danalyze=true

 -Dbpf-framework=false
)

%cross_meson "${CONFIGURE_OPTS[@]}"
%cross_meson_build

%install
%cross_meson_install

# Remove all stock network configurations, as they can interfere
# with container networking by attempting to manage veth devices.
rm -f %{buildroot}%{_cross_libdir}/systemd/network/*

# Remove default, multi-user and graphical targets provided by systemd,
# we override default/multi-user in the release spec and graphical
# is never used
rm -f %{buildroot}%{_cross_libdir}/systemd/{system,user}/default.target
rm -f %{buildroot}%{_cross_libdir}/systemd/{system,user}/multi-user.target
rm -f %{buildroot}%{_cross_libdir}/systemd/{system,user}/graphical.target

# Ensure /proc/sys/fs/binfmt_misc mount is wanted by sysinit.target,
# since we exclude the automount unit.
ln -s  ../proc-sys-fs-binfmt_misc.mount \
  %{buildroot}%{_cross_unitdir}/sysinit.target.wants/proc-sys-fs-binfmt_misc.mount

install -d %{buildroot}%{_cross_datadir}/whippet/policies.d
install -p -m 0644 %{S:100} %{S:101} %{S:102} %{S:103} %{buildroot}%{_cross_datadir}/whippet/policies.d

# Remove any README files.
find %{buildroot} -type f -name README -print -delete

%files
%license LICENSE.GPL2 LICENSE.LGPL2.1
%{_cross_attribution_file}
%{_cross_bindir}/busctl
%{_cross_bindir}/journalctl
%{_cross_bindir}/systemctl
%{_cross_bindir}/systemd-analyze
%{_cross_bindir}/systemd-cat
%{_cross_bindir}/systemd-cgls
%{_cross_bindir}/systemd-cgtop
%{_cross_bindir}/systemd-creds
%{_cross_bindir}/systemd-dissect
%{_cross_bindir}/systemd-delta
%{_cross_bindir}/systemd-detect-virt
%{_cross_bindir}/systemd-escape
%{_cross_bindir}/systemd-id128
%{_cross_bindir}/systemd-inhibit
%{_cross_bindir}/systemd-machine-id-setup
%{_cross_bindir}/systemd-mount
%{_cross_bindir}/systemd-notify
%{_cross_bindir}/systemd-nspawn
%{_cross_bindir}/systemd-path
%{_cross_bindir}/systemd-repart
%{_cross_bindir}/systemd-run
%{_cross_bindir}/systemd-socket-activate
%{_cross_bindir}/systemd-stdio-bridge
%{_cross_bindir}/systemd-sysusers
%{_cross_bindir}/systemd-tmpfiles
%{_cross_bindir}/systemd-umount
%{_cross_bindir}/udevadm
%{_cross_bindir}/loginctl

%{_cross_sbindir}/halt
%{_cross_sbindir}/init
%{_cross_sbindir}/poweroff
%{_cross_sbindir}/reboot
%{_cross_sbindir}/shutdown

%{_cross_libdir}/libsystemd.so.*
%{_cross_libdir}/libudev.so.*

%dir %{_cross_libdir}/systemd
%{_cross_libdir}/systemd/libsystemd-core-*.so
%{_cross_libdir}/systemd/libsystemd-shared-*.so
%{_cross_libdir}/systemd/systemd
%{_cross_libdir}/systemd/systemd-ac-power
%{_cross_libdir}/systemd/systemd-boot-check-no-failures
%{_cross_libdir}/systemd/systemd-cgroups-agent
%{_cross_libdir}/systemd/systemd-fsck
%{_cross_libdir}/systemd/systemd-growfs
%{_cross_libdir}/systemd/systemd-journald
%{_cross_libdir}/systemd/systemd-logind
%{_cross_libdir}/systemd/systemd-makefs
%{_cross_libdir}/systemd/systemd-modules-load
%{_cross_libdir}/systemd/systemd-network-generator
%{_cross_libdir}/systemd/systemd-pstore
%{_cross_libdir}/systemd/systemd-random-seed
%{_cross_libdir}/systemd/systemd-remount-fs
%{_cross_libdir}/systemd/systemd-shutdown
%{_cross_libdir}/systemd/systemd-sleep
%{_cross_libdir}/systemd/systemd-socket-proxyd
%{_cross_libdir}/systemd/systemd-sysctl
%{_cross_libdir}/systemd/systemd-sysroot-fstab-check
%{_cross_libdir}/systemd/systemd-udevd

%dir %{_cross_libdir}/systemd/system-preset
%{_cross_libdir}/systemd/system-preset/90-systemd.preset

%dir %{_cross_libdir}/systemd/system-shutdown
%dir %{_cross_libdir}/systemd/system-sleep


%dir %{_cross_libdir}/modprobe.d
%{_cross_libdir}/modprobe.d/systemd.conf


%dir %{_cross_sysctldir}
%{_cross_sysctldir}/50-default.conf
%{_cross_sysctldir}/50-pid-max.conf

%dir %{_cross_unitdir}
%{_cross_unitdir}/basic.target
%{_cross_unitdir}/blockdev@.target
%{_cross_unitdir}/boot-complete.target
%{_cross_unitdir}/ctrl-alt-del.target
%{_cross_unitdir}/dev-hugepages.mount
%{_cross_unitdir}/dev-mqueue.mount
%{_cross_unitdir}/exit.target
%{_cross_unitdir}/factory-reset.target
%{_cross_unitdir}/final.target
%{_cross_unitdir}/first-boot-complete.target
%{_cross_unitdir}/getty-pre.target
%{_cross_unitdir}/getty.target
%{_cross_unitdir}/halt.target
%{_cross_unitdir}/kexec.target
%{_cross_unitdir}/kmod-static-nodes.service
%{_cross_unitdir}/ldconfig.service
%{_cross_unitdir}/local-fs-pre.target
%{_cross_unitdir}/local-fs.target
%dir %{_cross_unitdir}/local-fs.target.wants
%{_cross_unitdir}/local-fs.target.wants/tmp.mount
%{_cross_unitdir}/modprobe@.service
%dir %{_cross_unitdir}/multi-user.target.wants
%{_cross_unitdir}/multi-user.target.wants/getty.target
%{_cross_unitdir}/multi-user.target.wants/systemd-logind.service
%{_cross_unitdir}/network-online.target
%{_cross_unitdir}/network-pre.target
%{_cross_unitdir}/network.target
%{_cross_unitdir}/nss-lookup.target
%{_cross_unitdir}/nss-user-lookup.target
%{_cross_unitdir}/paths.target
%{_cross_unitdir}/poweroff.target
%{_cross_unitdir}/proc-sys-fs-binfmt_misc.mount
%{_cross_unitdir}/reboot.target
%{_cross_unitdir}/rpcbind.target
%{_cross_unitdir}/shutdown.target
%{_cross_unitdir}/sigpwr.target
%{_cross_unitdir}/sleep.target
%{_cross_unitdir}/slices.target
%{_cross_unitdir}/sockets.target
%dir %{_cross_unitdir}/sockets.target.wants
%{_cross_unitdir}/sockets.target.wants/systemd-journald-audit.socket
%{_cross_unitdir}/sockets.target.wants/systemd-journald-dev-log.socket
%{_cross_unitdir}/sockets.target.wants/systemd-journald.socket
%{_cross_unitdir}/sockets.target.wants/systemd-udevd-control.socket
%{_cross_unitdir}/sockets.target.wants/systemd-udevd-kernel.socket
%{_cross_unitdir}/suspend.target
%{_cross_unitdir}/swap.target
%{_cross_unitdir}/sys-fs-fuse-connections.mount
%{_cross_unitdir}/sys-kernel-config.mount
%{_cross_unitdir}/sys-kernel-debug.mount
%{_cross_unitdir}/sys-kernel-tracing.mount
%{_cross_unitdir}/sysinit.target
%dir %{_cross_unitdir}/sysinit.target.wants
%{_cross_unitdir}/sysinit.target.wants/dev-hugepages.mount
%{_cross_unitdir}/sysinit.target.wants/dev-mqueue.mount
%{_cross_unitdir}/sysinit.target.wants/kmod-static-nodes.service
%{_cross_unitdir}/sysinit.target.wants/ldconfig.service
%{_cross_unitdir}/sysinit.target.wants/proc-sys-fs-binfmt_misc.mount
%{_cross_unitdir}/sysinit.target.wants/sys-fs-fuse-connections.mount
%{_cross_unitdir}/sysinit.target.wants/sys-kernel-config.mount
%{_cross_unitdir}/sysinit.target.wants/sys-kernel-debug.mount
%{_cross_unitdir}/sysinit.target.wants/sys-kernel-tracing.mount
%{_cross_unitdir}/sysinit.target.wants/systemd-journal-catalog-update.service
%{_cross_unitdir}/sysinit.target.wants/systemd-journal-flush.service
%{_cross_unitdir}/sysinit.target.wants/systemd-journald.service
%{_cross_unitdir}/sysinit.target.wants/systemd-machine-id-commit.service
%{_cross_unitdir}/sysinit.target.wants/systemd-modules-load.service
%{_cross_unitdir}/sysinit.target.wants/systemd-random-seed.service
%{_cross_unitdir}/sysinit.target.wants/systemd-repart.service
%{_cross_unitdir}/sysinit.target.wants/systemd-sysctl.service
%{_cross_unitdir}/sysinit.target.wants/systemd-sysusers.service
%{_cross_unitdir}/sysinit.target.wants/systemd-tmpfiles-setup-dev.service
%{_cross_unitdir}/sysinit.target.wants/systemd-tmpfiles-setup.service
%{_cross_unitdir}/sysinit.target.wants/systemd-udev-trigger.service
%{_cross_unitdir}/sysinit.target.wants/systemd-udevd.service
%{_cross_unitdir}/syslog.socket
%{_cross_unitdir}/systemd-boot-check-no-failures.service
%{_cross_unitdir}/systemd-exit.service
%{_cross_unitdir}/systemd-fsck-root.service
%{_cross_unitdir}/systemd-fsck@.service
%{_cross_unitdir}/systemd-halt.service
%{_cross_unitdir}/systemd-journal-catalog-update.service
%{_cross_unitdir}/systemd-journal-flush.service
%{_cross_unitdir}/systemd-journald-audit.socket
%{_cross_unitdir}/systemd-journald-dev-log.socket
%{_cross_unitdir}/systemd-journald-varlink@.socket
%{_cross_unitdir}/systemd-journald.service
%{_cross_unitdir}/systemd-journald.socket
%{_cross_unitdir}/systemd-journald@.service
%{_cross_unitdir}/systemd-journald@.socket
%{_cross_unitdir}/systemd-kexec.service
%{_cross_unitdir}/systemd-logind.service
%{_cross_unitdir}/systemd-machine-id-commit.service
%{_cross_unitdir}/systemd-modules-load.service
%{_cross_unitdir}/systemd-network-generator.service
%{_cross_unitdir}/systemd-nspawn@.service
%{_cross_unitdir}/systemd-poweroff.service
%{_cross_unitdir}/systemd-pstore.service
%{_cross_unitdir}/systemd-random-seed.service
%{_cross_unitdir}/systemd-reboot.service
%{_cross_unitdir}/systemd-remount-fs.service
%{_cross_unitdir}/systemd-suspend.service
%{_cross_unitdir}/systemd-sysctl.service
%{_cross_unitdir}/systemd-sysusers.service
%{_cross_unitdir}/systemd-tmpfiles-clean.service
%{_cross_unitdir}/systemd-tmpfiles-clean.timer
%{_cross_unitdir}/systemd-tmpfiles-setup-dev.service
%{_cross_unitdir}/systemd-tmpfiles-setup.service
%{_cross_unitdir}/systemd-udev-settle.service
%{_cross_unitdir}/systemd-udev-trigger.service
%{_cross_unitdir}/systemd-udevd-control.socket
%{_cross_unitdir}/systemd-udevd-kernel.socket
%{_cross_unitdir}/systemd-udevd.service
%{_cross_unitdir}/time-set.target
%{_cross_unitdir}/time-sync.target
%{_cross_unitdir}/timers.target
%dir %{_cross_unitdir}/timers.target.wants
%{_cross_unitdir}/timers.target.wants/systemd-tmpfiles-clean.timer
%{_cross_unitdir}/tmp.mount
%{_cross_unitdir}/umount.target
%{_cross_unitdir}/dbus-org.freedesktop.login1.service

# Exclude desktop related targets.
%exclude %{_cross_unitdir}/bluetooth.target
%exclude %{_cross_unitdir}/printer.target
%exclude %{_cross_unitdir}/smartcard.target
%exclude %{_cross_unitdir}/sound.target
%exclude %{_cross_unitdir}/usb-gadget.target

# Exclude remote filesystem targets.
%exclude %{_cross_unitdir}/remote-fs-pre.target
%exclude %{_cross_unitdir}/remote-fs.target
%exclude %{_cross_unitdir}/remote-cryptsetup.target
%exclude %{_cross_unitdir}/remote-veritysetup.target

# Exclude user-related functionality.
%exclude %{_cross_unitdir}/user-runtime-dir@.service
%exclude %{_cross_unitdir}/user@.service
%exclude %{_cross_unitdir}/user@.service.d
%exclude %{_cross_unitdir}/user@0.service.d
%exclude %{_cross_unitdir}/user-.slice.d/10-defaults.conf
%exclude %{_cross_unitdir}/user.slice
%exclude %{_cross_userunitdir}
%exclude %{_cross_libdir}/systemd/systemd-user-runtime-dir
%exclude %{_cross_libdir}/systemd/user-preset/90-systemd.preset

# Exclude units related to the initrd.
%exclude %{_cross_unitdir}/initrd-root-device.target.wants
%exclude %{_cross_unitdir}/initrd-root-fs.target.wants

# Exclude repart service since we have custom repart logic.
%exclude %{_cross_unitdir}/systemd-repart.service

# Exclude upstream binfmt functionality.
%exclude %{_cross_libdir}/systemd/systemd-binfmt
%exclude %{_cross_unitdir}/systemd-binfmt.service
%exclude %{_cross_unitdir}/proc-sys-fs-binfmt_misc.automount
%exclude %{_cross_unitdir}/sysinit.target.wants/proc-sys-fs-binfmt_misc.automount
%exclude %{_cross_unitdir}/sysinit.target.wants/systemd-binfmt.service

# Exclude functionality related to offline updates.
%exclude %{_cross_libdir}/systemd/systemd-update-done
%exclude %{_cross_libdir}/systemd/systemd-update-helper
%exclude %{_cross_systemdgeneratordir}/systemd-system-update-generator
%exclude %{_cross_unitdir}/sysinit.target.wants/systemd-update-done.service
%exclude %{_cross_unitdir}/system-update-cleanup.service
%exclude %{_cross_unitdir}/system-update-pre.target
%exclude %{_cross_unitdir}/system-update.target
%exclude %{_cross_unitdir}/systemd-update-done.service

%dir %{_cross_libdir}/udev
%{_cross_libdir}/udev/ata_id
%{_cross_libdir}/udev/cdrom_id
%{_cross_libdir}/udev/dmi_memory_id
%{_cross_libdir}/udev/fido_id
%{_cross_libdir}/udev/mtd_probe
%{_cross_libdir}/udev/scsi_id
%exclude %{_cross_libdir}/udev/v4l_id

%dir %{_cross_udevrulesdir}
%{_cross_udevrulesdir}/50-udev-default.rules
%{_cross_udevrulesdir}/60-autosuspend.rules
%{_cross_udevrulesdir}/60-block.rules
%{_cross_udevrulesdir}/60-cdrom_id.rules
%{_cross_udevrulesdir}/60-drm.rules
%{_cross_udevrulesdir}/60-evdev.rules
%{_cross_udevrulesdir}/60-fido-id.rules
%{_cross_udevrulesdir}/60-infiniband.rules
%{_cross_udevrulesdir}/60-input-id.rules
%{_cross_udevrulesdir}/60-persistent-input.rules
%{_cross_udevrulesdir}/60-persistent-storage-tape.rules
%{_cross_udevrulesdir}/60-persistent-storage.rules
%{_cross_udevrulesdir}/60-sensor.rules
%{_cross_udevrulesdir}/60-serial.rules
%{_cross_udevrulesdir}/64-btrfs.rules
%{_cross_udevrulesdir}/70-memory.rules
%{_cross_udevrulesdir}/70-power-switch.rules
%{_cross_udevrulesdir}/75-net-description.rules
%{_cross_udevrulesdir}/75-probe_mtd.rules
%{_cross_udevrulesdir}/80-drivers.rules
%{_cross_udevrulesdir}/80-net-setup-link.rules
%{_cross_udevrulesdir}/81-net-dhcp.rules
%{_cross_udevrulesdir}/99-systemd.rules

# Exclude desktop-related device rules.
%exclude %{_cross_udevrulesdir}/60-persistent-alsa.rules
%exclude %{_cross_udevrulesdir}/60-persistent-v4l.rules
%exclude %{_cross_udevrulesdir}/70-camera.rules
%exclude %{_cross_udevrulesdir}/70-joystick.rules
%exclude %{_cross_udevrulesdir}/70-mouse.rules
%exclude %{_cross_udevrulesdir}/70-touchpad.rules
%exclude %{_cross_udevrulesdir}/70-uaccess.rules
%exclude %{_cross_udevrulesdir}/71-seat.rules
%exclude %{_cross_udevrulesdir}/73-seat-late.rules
%exclude %{_cross_udevrulesdir}/78-sound-card.rules

%dir %{_cross_sysusersdir}
%{_cross_sysusersdir}/basic.conf
%{_cross_sysusersdir}/systemd-journal.conf

%dir %{_cross_tmpfilesdir}
%{_cross_tmpfilesdir}/etc.conf
%{_cross_tmpfilesdir}/home.conf
%{_cross_tmpfilesdir}/journal-nocow.conf
%{_cross_tmpfilesdir}/provision.conf
%{_cross_tmpfilesdir}/static-nodes-permissions.conf
%{_cross_tmpfilesdir}/systemd-pstore.conf
%{_cross_tmpfilesdir}/systemd-tmp.conf
%{_cross_tmpfilesdir}/systemd.conf
%{_cross_tmpfilesdir}/tmp.conf
%{_cross_tmpfilesdir}/var.conf
%exclude %{_cross_tmpfilesdir}/x11.conf

%{_cross_datadir}/dbus-1/system.d/org.freedesktop.login1.conf
%{_cross_datadir}/dbus-1/system.d/org.freedesktop.systemd1.conf
%exclude %{_cross_datadir}/dbus-1/system-services
%exclude %{_cross_datadir}/dbus-1/services/org.freedesktop.systemd1.service

%{_cross_datadir}/whippet/policies.d/org.freedesktop.login1.toml
%{_cross_datadir}/whippet/policies.d/org.freedesktop.systemd1.toml

%dir %{_cross_factorydir}
%exclude %{_cross_factorydir}%{_cross_sysconfdir}/issue
%{_cross_factorydir}%{_cross_sysconfdir}/locale.conf
%exclude %{_cross_factorydir}%{_cross_sysconfdir}/nsswitch.conf
%exclude %{_cross_factorydir}%{_cross_sysconfdir}/pam.d
%exclude %{_cross_factorydir}%{_cross_sysconfdir}/pam.d/other
%exclude %{_cross_factorydir}%{_cross_sysconfdir}/pam.d/system-auth

%dir %{_cross_journalcatalogdir}
%{_cross_journalcatalogdir}/systemd.catalog
%exclude %{_cross_journalcatalogdir}/systemd.be.catalog
%exclude %{_cross_journalcatalogdir}/systemd.be@latin.catalog
%exclude %{_cross_journalcatalogdir}/systemd.bg.catalog
%exclude %{_cross_journalcatalogdir}/systemd.da.catalog
%exclude %{_cross_journalcatalogdir}/systemd.de.catalog
%exclude %{_cross_journalcatalogdir}/systemd.fr.catalog
%exclude %{_cross_journalcatalogdir}/systemd.hr.catalog
%exclude %{_cross_journalcatalogdir}/systemd.hu.catalog
%exclude %{_cross_journalcatalogdir}/systemd.it.catalog
%exclude %{_cross_journalcatalogdir}/systemd.ko.catalog
%exclude %{_cross_journalcatalogdir}/systemd.pl.catalog
%exclude %{_cross_journalcatalogdir}/systemd.pt_BR.catalog
%exclude %{_cross_journalcatalogdir}/systemd.ru.catalog
%exclude %{_cross_journalcatalogdir}/systemd.sr.catalog
%exclude %{_cross_journalcatalogdir}/systemd.zh_CN.catalog
%exclude %{_cross_journalcatalogdir}/systemd.zh_TW.catalog

%dir %{_cross_systemdgeneratordir}
%{_cross_systemdgeneratordir}/systemd-fstab-generator
%{_cross_systemdgeneratordir}/systemd-run-generator

%exclude %{_cross_datadir}/polkit-1
%exclude %{_cross_docdir}
%exclude %{_cross_libdir}/pam.d/systemd-user
%exclude %{_cross_sysconfdir}/systemd/
%exclude %{_cross_sysconfdir}/udev/
%exclude %{_cross_sysconfdir}/X11
%exclude %{_cross_sysconfdir}/xdg

%files devel
%{_cross_libdir}/libsystemd.so
%{_cross_libdir}/libudev.so
%{_cross_includedir}/libudev.h
%dir %{_cross_includedir}/systemd
%{_cross_includedir}/systemd/*.h
%{_cross_pkgconfigdir}/*.pc
%exclude %{_cross_libdir}/rpm/macros.d

%files console
%{_cross_bindir}/systemd-ask-password
%{_cross_bindir}/systemd-tty-ask-password-agent
%{_cross_libdir}/systemd/systemd-sulogin-shell
%{_cross_libdir}/systemd/systemd-reply-password
%{_cross_systemdgeneratordir}/systemd-debug-generator
%{_cross_systemdgeneratordir}/systemd-getty-generator
%{_cross_unitdir}/autovt@.service
%{_cross_unitdir}/console-getty.service
%{_cross_unitdir}/container-getty@.service
%{_cross_unitdir}/debug-shell.service
%{_cross_unitdir}/emergency.service
%{_cross_unitdir}/emergency.target
%{_cross_unitdir}/getty@.service
%{_cross_unitdir}/rescue.service
%{_cross_unitdir}/rescue.target
%{_cross_unitdir}/serial-getty@.service
%{_cross_unitdir}/systemd-ask-password-console.service
%{_cross_unitdir}/systemd-ask-password-console.path
%{_cross_unitdir}/systemd-ask-password-wall.path
%{_cross_unitdir}/systemd-ask-password-wall.service
%{_cross_unitdir}/sysinit.target.wants/systemd-ask-password-console.path
%{_cross_unitdir}/multi-user.target.wants/systemd-ask-password-wall.path

%files networkd
%{_cross_bindir}/networkctl
%dir %{_cross_libdir}/systemd/network
%{_cross_libdir}/systemd/systemd-networkd
%{_cross_libdir}/systemd/systemd-networkd-wait-online
%{_cross_sysusersdir}/systemd-network.conf
%{_cross_tmpfilesdir}/systemd-network.conf
%{_cross_unitdir}/systemd-networkd.service
%{_cross_unitdir}/systemd-networkd-wait-online.service
%{_cross_unitdir}/systemd-networkd-wait-online@.service
%{_cross_unitdir}/systemd-networkd.socket
%{_cross_datadir}/dbus-1/system.d/org.freedesktop.network1.conf
%{_cross_datadir}/whippet/policies.d/org.freedesktop.network1.toml

%files resolved
%{_cross_bindir}/resolvectl
%{_cross_libdir}/libnss_resolve.so.*
%{_cross_libdir}/systemd/resolv.conf
%{_cross_libdir}/systemd/systemd-resolved
%{_cross_sysusersdir}/systemd-resolve.conf
%{_cross_tmpfilesdir}/systemd-resolve.conf
%{_cross_unitdir}/systemd-resolved.service
%{_cross_datadir}/dbus-1/system.d/org.freedesktop.resolve1.conf
%{_cross_datadir}/whippet/policies.d/org.freedesktop.resolve1.toml
%exclude %{_cross_bindir}/systemd-resolve
%exclude %{_cross_sbindir}/resolvconf

%files cryptsetup
%{_cross_bindir}/systemd-cryptenroll
%{_cross_libdir}/cryptsetup/libcryptsetup-token-systemd-tpm2.so
%{_cross_libdir}/systemd/systemd-cryptsetup
%{_cross_libdir}/systemd/systemd-integritysetup
%{_cross_libdir}/systemd/systemd-veritysetup
%{_cross_systemdgeneratordir}/systemd-cryptsetup-generator
%{_cross_systemdgeneratordir}/systemd-integritysetup-generator
%{_cross_systemdgeneratordir}/systemd-veritysetup-generator
%{_cross_unitdir}/cryptsetup.target
%{_cross_unitdir}/cryptsetup-pre.target
%{_cross_unitdir}/integritysetup.target
%{_cross_unitdir}/integritysetup-pre.target
%{_cross_unitdir}/veritysetup.target
%{_cross_unitdir}/veritysetup-pre.target
%{_cross_unitdir}/*cryptsetup.slice
%{_cross_unitdir}/sysinit.target.wants/cryptsetup.target
%{_cross_unitdir}/sysinit.target.wants/integritysetup.target
%{_cross_unitdir}/sysinit.target.wants/veritysetup.target
