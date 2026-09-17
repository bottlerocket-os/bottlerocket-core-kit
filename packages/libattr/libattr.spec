Name: %{_cross_os}libattr
Version: 2.6.0
Release: 1%{?dist}
Epoch: 1
Summary: Library for extended attribute support
License: LGPL-2.1-or-later
URL: https://savannah.nongnu.org/projects/attr
Source0: https://download-mirror.savannah.gnu.org/releases/attr/attr-%{version}.tar.xz
Source1: https://download-mirror.savannah.gnu.org/releases/attr/attr-%{version}.tar.xz.sig
Source2: gpgkey-259B3792B3D6D319212CC4DCD5BF9FEB0313653A.asc
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for extended attribute support
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n attr-%{version} -p1

%build
%cross_configure
%force_disable_rpath

%make_build

%install
%make_install

%files
%license doc/COPYING.LGPL
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*
%exclude %{_cross_sysconfdir}/xattr.conf
%exclude %{_cross_bindir}
%exclude %{_cross_docdir}
%exclude %{_cross_localedir}
%exclude %{_cross_mandir}

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.so
%dir %{_cross_includedir}/attr
%{_cross_includedir}/attr/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
