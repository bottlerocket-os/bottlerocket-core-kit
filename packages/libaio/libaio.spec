# Disable LTO since it's not compatible with libaio's symbol versioning.
%global _cross_cflags %{_cross_cflags} -fno-lto

Name: %{_cross_os}libaio
Version: 0.3.113
Release: 1%{?dist}
Summary: Library for asynchronous I/O access
License: LGPL-2.0-or-later
URL: http://releases.pagure.org/libaio
Source0: %{url}/libaio-%{version}.tar.gz

BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for asynchronous I/O access
Requires: %{name}

%description devel
%{summary}.

%prep
%autosetup -n libaio-%{version} -p1

%build
%set_cross_build_flags
%make_build CC="%{_cross_target}-gcc"
%cross_generate_sbom

%install
make \
  DESTDIR=%{buildroot} \
  prefix=%{_cross_prefix} \
  libdir=%{_cross_libdir} \
  usrlibdir=%{_cross_libdir} \
  includedir=%{_cross_includedir} \
  install
%cross_install_sbom

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_libdir}/libaio.so.*
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json

%files devel
%{_cross_libdir}/libaio.a
%{_cross_libdir}/libaio.so
%{_cross_includedir}/libaio.h
