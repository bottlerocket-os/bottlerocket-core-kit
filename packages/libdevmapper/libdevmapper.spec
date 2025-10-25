Name: %{_cross_os}libdevmapper
Version: 2.03.35
Release: 1%{?dist}
Summary: Library for device mapper
License: LGPL-2.1-only
URL: https://sourceware.org/lvm2
Source0: https://sourceware.org/pub/lvm2/releases/LVM2.%{version}.tgz
Source1: https://sourceware.org/pub/lvm2/releases/LVM2.%{version}.tgz.asc
Source2: gpgkey-D501A478440AE2FD130A1BE8B9112431E509039F.asc

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libaio-devel
BuildRequires: %{_cross_os}libblkid-devel
BuildRequires: %{_cross_os}libselinux-devel
BuildRequires: %{_cross_os}libudev-devel
Requires: %{_cross_os}libaio
Requires: %{_cross_os}libblkid
Requires: %{_cross_os}libselinux
Requires: %{_cross_os}libudev

%description
%{summary}.

%package devel
Summary: Files for development using the library for device mapper
Requires: %{name}
Requires: %{_cross_os}libudev-devel

%description devel
%{summary}.

%package -n %{_cross_os}dmsetup
Summary: Utility for managing device mapper devices
License: GPL-2.0-only
Requires: %{name}

%description -n %{_cross_os}dmsetup
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n LVM2.%{version} -p1

%build
%cross_configure \
  --prefix="%{_cross_prefix}" \
  --disable-dbus-service \
  --disable-notify-dbus \
  --disable-dmeventd \
  --disable-dmfilemapd \
  --disable-fsadm \
  --disable-lvmimportvdo \
  --disable-lvmpolld \
  --disable-use-lvmlockd \
  --disable-use-lvmpolld \
  --disable-readline \
  --enable-pkgconfig \
  --enable-selinux \
  --enable-udev_rules \
  --enable-udev_sync \
  --with-user= \
  --with-group= \
  --with-device-uid=0 \
  --with-device-gid=0 \
  --with-device-mode=0660 \
  --with-cache=none \
  --with-integrity=none \
  --with-mirrors=none \
  --with-thin=none \
  --with-snapshots=none \
  --with-vdo=none \
  --with-writecache=none \
  %{nil}

%make_build device-mapper

%install
make install_device-mapper DESTDIR=%{buildroot} INSTALL="/usr/bin/install -p"
find %{buildroot} -type f -executable -exec chmod u+w {} +

%files
%license COPYING.LIB
%{_cross_attribution_file}
%{_cross_libdir}/libdevmapper.so.*
%exclude %{_cross_mandir}

%files devel
%{_cross_libdir}/libdevmapper.so
%{_cross_includedir}/libdevmapper.h
%{_cross_pkgconfigdir}/devmapper.pc

%files -n %{_cross_os}dmsetup
%license COPYING
%{_cross_sbindir}/blkdeactivate
%{_cross_sbindir}/dmsetup
%{_cross_sbindir}/dmstats
%{_cross_udevrulesdir}/10-dm.rules
%{_cross_udevrulesdir}/13-dm-disk.rules
%{_cross_udevrulesdir}/95-dm-notify.rules
