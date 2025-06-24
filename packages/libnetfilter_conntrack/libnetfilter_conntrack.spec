Name: %{_cross_os}libnetfilter_conntrack
Version: 1.1.0
Release: 1%{?dist}
Epoch: 1
Summary: Library for netfilter conntrack
License: GPL-2.0-or-later
URL: http://netfilter.org
Source0: https://netfilter.org/projects/libnetfilter_conntrack/files/libnetfilter_conntrack-%{version}.tar.xz
Source1: https://netfilter.org/projects/libnetfilter_conntrack/files/libnetfilter_conntrack-%{version}.tar.xz.sig
Source2: gpgkey-37D964ACC04981C75500FB9BD55D978A8A1420E4.asc

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libmnl-devel
BuildRequires: %{_cross_os}libnfnetlink-devel
Requires: %{_cross_os}libmnl
Requires: %{_cross_os}libnfnetlink

%description
%{summary}.

%package devel
Summary: Files for development using the library for netfilter conntrack
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n libnetfilter_conntrack-%{version} -p1

%build
%cross_configure \
  --disable-rpath \
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
%dir %{_cross_includedir}/libnetfilter_conntrack
%{_cross_includedir}/libnetfilter_conntrack/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
