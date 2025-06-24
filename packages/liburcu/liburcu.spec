Name: %{_cross_os}liburcu
Version: 0.15.3
Release: 1%{?dist}
Epoch: 1
Summary: Library for userspace RCU
License: LGPL-2.1-only AND GPL-2.0-or-later AND MIT
URL: http://liburcu.org
Source0: http://lttng.org/files/urcu/userspace-rcu-%{version}.tar.bz2
Source1: http://lttng.org/files/urcu/userspace-rcu-%{version}.tar.bz2.asc
Source2: gpgkey-2A0B4ED915F2D3FA45F5B16217280A9781186ACF.asc
Patch0001: 0001-build-do-not-build-examples.patch

BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for userspace RCU
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n userspace-rcu-%{version} -p1

%build
autoreconf -fi
%cross_configure --disable-static

%force_disable_rpath

%make_build

%cross_generate_sbom

%install
%make_install

%cross_install_sbom

%files
%license LICENSE.md lgpl-relicensing.md
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json

%{_cross_libdir}/liburcu.so.8*
%{_cross_libdir}/liburcu-common.so.8*

%exclude %{_cross_libdir}/liburcu-bp.so.8*
%exclude %{_cross_libdir}/liburcu-cds.so.8*
%exclude %{_cross_libdir}/liburcu-mb.so.8*
%exclude %{_cross_libdir}/liburcu-memb.so.8*
%exclude %{_cross_libdir}/liburcu-qsbr.so.8*
%exclude %{_cross_docdir}

%files devel
%{_cross_includedir}/*
%{_cross_libdir}/liburcu-common.so
%{_cross_libdir}/liburcu.so
%{_cross_libdir}/pkgconfig/liburcu.pc

%exclude %{_cross_libdir}/pkgconfig/liburcu-bp.pc
%exclude %{_cross_libdir}/pkgconfig/liburcu-cds.pc
%exclude %{_cross_libdir}/pkgconfig/liburcu-mb.pc
%exclude %{_cross_libdir}/pkgconfig/liburcu-memb.pc
%exclude %{_cross_libdir}/pkgconfig/liburcu-qsbr.pc
%exclude %{_cross_libdir}/liburcu-bp.so
%exclude %{_cross_libdir}/liburcu-cds.so
%exclude %{_cross_libdir}/liburcu-common.so
%exclude %{_cross_libdir}/liburcu-mb.so
%exclude %{_cross_libdir}/liburcu-memb.so
%exclude %{_cross_libdir}/liburcu-qsbr.so
