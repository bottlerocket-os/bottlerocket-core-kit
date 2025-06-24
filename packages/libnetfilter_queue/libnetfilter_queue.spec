Name: %{_cross_os}libnetfilter_queue
Version: 1.0.5
Release: 1%{?dist}
Epoch: 1
Summary: Library for netfilter queue
License: GPL-2.0-or-later
URL: http://netfilter.org
Source0: https://netfilter.org/projects/libnetfilter_queue/files/libnetfilter_queue-%{version}.tar.bz2
Source1: https://netfilter.org/projects/libnetfilter_queue/files/libnetfilter_queue-%{version}.tar.bz2.sig
Source2: gpgkey-C09DB2063F1D7034BA6152ADAB4655A126D292E4.asc
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libmnl-devel
BuildRequires: %{_cross_os}libnfnetlink-devel
Requires: %{_cross_os}libmnl
Requires: %{_cross_os}libnfnetlink

%description
%{summary}.

%package devel
Summary: Files for development using the library for netfilter queue
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n libnetfilter_queue-%{version} -p1

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
%dir %{_cross_includedir}/libnetfilter_queue
%{_cross_includedir}/libnetfilter_queue/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
