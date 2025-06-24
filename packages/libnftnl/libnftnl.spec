Name: %{_cross_os}libnftnl
Version: 1.2.9
Release: 1%{?dist}
Epoch: 1
Summary: Library for nftables netlink
License: GPL-2.0-or-later AND GPL-2.0-only
URL: http://netfilter.org/projects/libnftnl/
Source0: https://netfilter.org/projects/libnftnl/files/libnftnl-%{version}.tar.xz
Source1: https://netfilter.org/projects/libnftnl/files/libnftnl-%{version}.tar.xz.sig
Source2: gpgkey-8C5F7146A1757A65E2422A94D70D1A666ACF2B21.asc
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libmnl-devel
Requires: %{_cross_os}libmnl

%description
%{summary}.

%package devel
Summary: Files for development using the library for nftables netlink
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n libnftnl-%{version} -p1

%build
%cross_configure \
  --disable-silent-rules \
  --enable-static \
  --without-json-parsing \

%make_build
%cross_generate_sbom

%install
%make_install
%cross_install_sbom

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.so
%dir %{_cross_includedir}/libnftnl
%{_cross_includedir}/libnftnl/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
