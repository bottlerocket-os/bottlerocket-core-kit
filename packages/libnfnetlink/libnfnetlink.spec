Name: %{_cross_os}libnfnetlink
Version: 1.0.2
Release: 1%{?dist}
Epoch: 1
Summary: Library for netfilter netlink
License: GPL-2.0-only
URL: http://netfilter.org
Source0: http://netfilter.org/projects/libnfnetlink/files/libnfnetlink-%{version}.tar.bz2
Source1: http://netfilter.org/projects/libnfnetlink/files/libnfnetlink-%{version}.tar.bz2.sig
Source2: gpgkey-37D964ACC04981C75500FB9BD55D978A8A1420E4.asc
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for netfilter netlink
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n libnfnetlink-%{version} -p1

%build
%cross_configure \
  --enable-static

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
%dir %{_cross_includedir}/libnfnetlink
%{_cross_includedir}/libnfnetlink/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
