Name: %{_cross_os}readline
Version: 8.3
Release: 1%{?dist}
Summary: A library for editing typed command lines
License: GPL-3.0-or-later
URL: https://tiswww.case.edu/php/chet/readline/rltop.html
Source0: https://ftp.gnu.org/gnu/readline/readline-%{version}.tar.gz
Source1: https://ftp.gnu.org/gnu/readline/readline-%{version}.tar.gz.sig
Source2: gpgkey-7C0135FB088AAF6C66C650B9BB5869F064EA74AB.asc
Patch1: readline-8.3-shlib.patch

Patch1001: readline83-001
Patch1002: readline83-002
Patch1003: readline83-003
Patch1004: readline83-004
Patch1005: readline83-005
Patch1006: readline83-006

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libncurses-devel
Requires: %{_cross_os}libncurses

%description
%{summary}.

%package devel
Summary: Files for development using a library for editing typed command lines
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n readline-%{version} -N
%autopatch -p1 -m0001 -M1000
%autopatch -p0 -m1001

%build
%cross_configure --with-curses --disable-install-examples
%make_build

%install
%make_install

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*
%exclude %{_cross_infodir}
%exclude %{_cross_mandir}
%exclude %{_cross_datadir}/doc/readline/*

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.so
%dir %{_cross_includedir}/readline
%{_cross_includedir}/readline/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
