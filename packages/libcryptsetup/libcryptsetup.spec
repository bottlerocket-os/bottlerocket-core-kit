Name: %{_cross_os}libcryptsetup
Version: 2.8.1
Release: 1%{?dist}
Summary: Libraries for disk encryption support
License: GPL-2.0-or-later WITH cryptsetup-OpenSSL-exception AND LGPL-2.1-or-later WITH cryptsetup-OpenSSL-exception
URL: https://gitlab.com/cryptsetup/cryptsetup
Source0: https://www.kernel.org/pub/linux/utils/cryptsetup/v2.7/cryptsetup-%{version}.tar.xz
Source1: https://www.kernel.org/pub/linux/utils/cryptsetup/v2.7/cryptsetup-%{version}.tar.sign
Source2: gpgkey-2A2918243FDE46648D0686F9D9B0577BD93E98FC.asc

# AWS-LC is always in FIPS mode, which will prevent any use of the argon2 PBKDF.
# This patch allows argon2 usage unless the kernel is also in FIPS mode.
Patch0001: 0001-pbkdf-check-whether-FIPS-is-enabled-at-runtime.patch

# cryptsetup only depends on libcrypto, not libssl.
Patch0002: 0002-build-replace-openssl-with-libcrypto-in-pkgconfig.patch

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libblkid-devel
BuildRequires: %{_cross_os}libcrypto-devel
BuildRequires: %{_cross_os}libdevmapper-devel
BuildRequires: %{_cross_os}libjson-c-devel
BuildRequires: %{_cross_os}libpopt-devel
BuildRequires: %{_cross_os}libselinux-devel
BuildRequires: %{_cross_os}libuuid-devel

Requires: %{_cross_os}libblkid
Requires: %{_cross_os}libcrypto
Requires: %{_cross_os}libdevmapper
Requires: %{_cross_os}libjson-c
Requires: %{_cross_os}libselinux
Requires: %{_cross_os}libuuid

%description
%{summary}.

%package tools
Summary: Command line tools for the libraries for disk encryption support
Provides: %{_cross_os}cryptsetup
Requires: %{name}
Requires: %{_cross_os}libpopt
Requires: %{_cross_os}dmsetup

%description tools
%{summary}.

%package devel
Summary: Files for development using the libraries for disk encryption support
Requires: %{name}
Requires: %{_cross_os}libblkid-devel
Requires: %{_cross_os}libcrypto-devel
Requires: %{_cross_os}libdevmapper-devel
Requires: %{_cross_os}libjson-c-devel
Requires: %{_cross_os}libuuid-devel

%description devel
%{summary}.

%prep
%{gpgverify} --data=<(xzcat %{S:0}) --signature=%{S:1} --keyring=%{S:2}
%autosetup -n cryptsetup-%{version} -p1

%build
autoreconf -fi
%cross_configure \
  --disable-asciidoc \
  --disable-libargon2 \
  --disable-nls \
  --disable-pwquality \
  --disable-rpath \
  --disable-ssh-token \
  --disable-static \
  --enable-blkid \
  --enable-cryptsetup \
  --enable-fips \
  --enable-integritysetup \
  --enable-internal-argon2 \
  --enable-internal-sse-argon2 \
  --enable-selinux \
  --enable-udev \
  --enable-veritysetup \
  --with-crypto_backend=openssl \
  --with-luks2-pbkdf=pbkdf2 \
  --with-tmpfilesdir=%{_cross_libdir}/tmpfiles.d \
  %{nil}

%force_disable_rpath

%make_build

%install
%make_install

%files
%license COPYING docs/licenses/COPYING.LGPL-2.1-or-later-WITH-cryptsetup-OpenSSL-exception
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*
%{_cross_tmpfilesdir}/cryptsetup.conf
%exclude %{_cross_mandir}

%files tools
%{_cross_sbindir}/cryptsetup
%{_cross_sbindir}/integritysetup
%{_cross_sbindir}/veritysetup

%files devel
%{_cross_libdir}/*.so
%{_cross_includedir}/*.h
%{_cross_pkgconfigdir}/*.pc
