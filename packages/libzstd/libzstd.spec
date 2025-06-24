Name: %{_cross_os}libzstd
Version: 1.5.7
Release: 1%{?dist}
Epoch: 1
Summary: Library for Zstandard compression
License: BSD-3-Clause AND GPL-2.0-only
URL: https://github.com/facebook/zstd/
Source0: https://github.com/facebook/zstd/releases/download/v%{version}/zstd-%{version}.tar.gz
Source1: https://github.com/facebook/zstd/releases/download/v%{version}/zstd-%{version}.tar.gz.sig
Source2: gpgkey-4EF4AC63455FC9F4545D9B7DEF8FE99528B52FFD.asc
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for Zstandard compression
Requires: %{_cross_os}libzstd

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n zstd-%{version}

%global set_env \
%set_cross_build_flags \\\
export CC=%{_cross_target}-gcc \\\
export PREFIX=%{_cross_prefix} \\\
export DESTDIR=%{buildroot}%{_cross_rootdir} \\\
%{nil}

%build
%set_env
%make_build
%cross_generate_sbom

%install
%set_env
%make_install
%cross_install_sbom

%files
%license COPYING
%{_cross_libdir}/*.so.*
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%exclude %{_cross_bindir}
%exclude %{_cross_mandir}

%files devel
%{_cross_includedir}/*.h
%{_cross_libdir}/*.so
%{_cross_libdir}/*.a
%{_cross_libdir}/pkgconfig/*.pc
