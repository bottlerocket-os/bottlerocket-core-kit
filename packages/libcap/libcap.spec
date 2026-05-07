Name: %{_cross_os}libcap
Version: 2.78
Release: 1%{?dist}
Epoch: 1
Summary: Library for getting and setting POSIX.1e capabilities
License: GPL-2.0-only OR BSD-3-Clause
URL: https://sites.google.com/site/fullycapable/
Source0: https://git.kernel.org/pub/scm/libs/libcap/libcap.git/snapshot/libcap-%{version}.tar.gz
Source1: https://git.kernel.org/pub/scm/libs/libcap/libcap.git/snapshot/libcap-%{version}.tar.sign
Source2: gpgkey-38A644698C69787344E954CE29EE848AE2CCF3F4.asc
BuildRequires: libcap-devel
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libattr-devel
Requires: %{_cross_os}libattr

# Local changes.
Patch9001: 9001-dont-test-during-install.patch

%description
%{summary}.

%package devel
Summary: Files for development using the library for getting and setting POSIX.1e capabilities
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=<(zcat %{S:0}) --signature=%{S:1} --keyring=%{S:2}
%autosetup -n libcap-%{version} -p1

%global cross_make \
make\\\
  DESTDIR=%{buildroot}\\\
  CROSS_COMPILE="%{_cross_target}-"\\\
  CFLAGS="%{_cross_cflags} -fPIC"\\\
  LDFLAGS="%{_cross_ldflags}"\\\
  BUILD_CC="gcc"\\\
  BUILD_CFLAGS="%{__global_compiler_flags}"\\\
  BUILD_LDFLAGS="%{build_ldflags}"\\\
  prefix=%{_cross_prefix}\\\
  lib=%{_cross_lib}\\\
  LIBDIR=%{_cross_libdir}\\\
  SBINDIR=%{_cross_sbindir}\\\
  INCDIR=%{_cross_includedir}\\\
  MANDIR=%{_cross_mandir}\\\
  PKGCONFIGDIR=%{_cross_pkgconfigdir}\\\
  GOLANG=no\\\
  RAISE_SETFCAP=no\\\
  PAM_CAP=no\\\
%{nil}

%build
%{cross_make}

%install
%{cross_make} install

chmod +x %{buildroot}%{_cross_libdir}/*.so.*

%files
%license License
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*
%exclude %{_cross_mandir}
%exclude %{_cross_sbindir}

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.so
%{_cross_includedir}/sys/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
