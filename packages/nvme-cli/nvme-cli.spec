Name: %{_cross_os}nvme-cli
Version: 3.2
Release: 1%{?dist}
Epoch: 1
Summary: CLI to interact with NVMe devices
License: GPL-2.0-only AND GPL-2.0-or-later AND LGPL-2.1-or-later AND CC0-1.0 AND MIT AND LicenseRef-Gary-S-Brown-CRC32
URL: https://github.com/linux-nvme/nvme-cli
Source0: https://github.com/linux-nvme/nvme-cli/archive/v%{version}/nvme-cli-%{version}.tar.gz

# nvme-cli 3.x opens only /dev/<basename of the device argument>, so udev
# symlinks such as /dev/disk/by-ebs-id/* and /dev/disk/by-id/* fail with
# EINVAL. Drop this once upstream libnvme resolves symlinks itself.
Patch1001: 1001-libnvme-resolve-device-symlinks.patch

BuildRequires: meson
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libjson-c-devel
Requires: %{_cross_os}libjson-c

%description
%{summary}.

%prep
%autosetup -n nvme-cli-%{version} -p1

%build
CONFIGURE_OPTS=(
 -Ddocs=false
 -Dexamples=false
 -Dtests=false
 -Dpython=disabled
 -Djson-c=enabled

 # ghostdog runs `nvme id-ctrl`, the deprecated 2.x alias of `nvme id ctrl`
 -Ddeprecated-cmds=enabled

 -Dkmip=disabled
 -Dmdns=disabled
 -Dnvme-discoverd=disabled
 -Dnvmf-autoconnect=disabled

 -Dkeyutils=disabled
 -Dlibarchive=disabled
 -Dlibkmod=disabled
 -Dliburing=disabled
 -Dopenssl=disabled
)

%cross_meson "${CONFIGURE_OPTS[@]}"
%cross_meson_build

%install
%cross_meson_install
# This is a sample configuration file with comments with examples of how to
# configure NVMe over Fabrics connections
rm %{buildroot}%{_sysconfdir}/nvme/nvme-fabrics.conf.sample

%files
%license LICENSE LICENSES/CC0-1.0.txt LICENSES/GPL-2.0.txt LICENSES/GPL-2.0-only.txt LICENSES/GPL-2.0-or-later.txt LICENSES/LGPL-2.1-or-later.txt LICENSES/LicenseRef-Gary-S-Brown-CRC32.txt LICENSES/MIT.txt
%{_cross_attribution_file}
%{_cross_sbindir}/nvme
%{_cross_libdir}/libnvme3.so.*
%exclude %{_cross_includedir}
%exclude %{_cross_libdir}/libnvme3.so
%exclude %{_cross_libdir}/pkgconfig
%exclude %{_cross_udevrulesdir}
%exclude %{_cross_datadir}/bash-completion
%exclude %{_cross_datadir}/zsh

%changelog
