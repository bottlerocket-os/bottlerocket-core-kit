Name: %{_cross_os}libseccomp
Version: 2.6.0
Release: 1%{?dist}
Epoch: 1
Summary: Library for enhanced seccomp
License: LGPL-2.1-only
URL: https://github.com/seccomp/libseccomp
Source0: https://github.com/seccomp/libseccomp/releases/download/v%{version}/libseccomp-%{version}.tar.gz
Source1: https://github.com/seccomp/libseccomp/releases/download/v%{version}/libseccomp-%{version}.tar.gz.asc
Source2: gpgkey-47A68FCE37C7D7024FD65E11356CE62C2B524099.asc
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for enhanced seccomp
Requires: %{name}

%description devel
%{summary}.

%prep
%autosetup -n libseccomp-%{version} -p1

%build
%cross_configure

%force_disable_rpath

%make_build

%cross_generate_sbom

%install
%make_install

%cross_install_sbom

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%exclude %{_cross_bindir}/scmp_sys_resolver
%exclude %{_cross_mandir}

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.so
%{_cross_includedir}/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
