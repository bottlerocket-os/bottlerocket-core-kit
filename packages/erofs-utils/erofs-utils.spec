Name: %{_cross_os}erofs-utils
Version: 1.9.4
Release: 1%{?dist}
Summary: Utilities for managing the EROFS filesystem
License: MIT AND GPL-2.0-only AND (GPL-2.0-or-later OR MIT) AND (GPL-2.0-only OR BSD-2-Clause) AND (GPL-2.0-or-later OR BSD-2-Clause) AND Unlicense
URL: https://git.kernel.org/pub/scm/linux/kernel/git/xiang/erofs-utils.git
Source0: %{url}/snapshot/erofs-utils-%{version}.tar.gz

# Do not prefix binaries with the target name when cross-compiling.
Patch0001: 0001-build-disable-AC_CANONICAL_TARGET.patch

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libselinux-devel
BuildRequires: %{_cross_os}libuuid-devel
BuildRequires: %{_cross_os}libzstd-devel
BuildRequires: %{_cross_os}libz-devel
Requires: %{_cross_os}libselinux
Requires: %{_cross_os}libuuid
Requires: %{_cross_os}libz
Requires: %{_cross_os}libzstd

%description
%{summary}.

%prep
%autosetup -n erofs-utils-%{version} -p1

%build
autoreconf -fi
%cross_configure \
  --enable-multithreading \
  --disable-fanotify \
  --disable-fuse \
  --disable-lz4 \
  --disable-lzma \
  --disable-oci \
  --disable-s3 \
  --disable-ublk \
  --with-libzstd \
  --with-selinux \
  --with-uuid \
  --with-zlib \
  --without-json-c \
  --without-libcurl \
  --without-libdeflate \
  --without-libnl3 \
  --without-libxml2 \
  --without-openssl \
  --without-qpl \
  --without-xxhash \
  %{nil}

%make_build

%install
%make_install

%files
%license LICENSES/GPL-2.0 LICENSES/MIT
%{_cross_attribution_file}
%{_cross_bindir}/dump.erofs
%{_cross_bindir}/fsck.erofs
%{_cross_bindir}/mkfs.erofs
%exclude %{_cross_sbindir}/mount.erofs
%exclude %{_cross_mandir}
