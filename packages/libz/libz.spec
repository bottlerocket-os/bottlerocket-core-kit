Name: %{_cross_os}libz
Version: 2.3.2
Release: 1%{?dist}
Epoch: 1
Summary: Library for zlib compression
URL: https://github.com/zlib-ng/zlib-ng
License: Zlib
Source0: https://github.com/zlib-ng/zlib-ng/archive/%{version}/zlib-ng-%{version}.tar.gz
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for zlib compression
Requires: %{name}

%description devel
%{summary}.

%prep
%autosetup -n zlib-ng-%{version} -p1

# Sets cross build flags, target cross compiler, and env variables
# required to `make install` libz
%global set_env \
%set_cross_build_flags \\\
export CFLAGS="${CFLAGS} -fPIC" \\\
export CROSS_PREFIX="%{_cross_target}-" \\\
%{nil}

%build
%set_env
./configure \
  --prefix='%{_cross_prefix}' \
  --without-new-strategies \
  --zlib-compat
%make_build

%install
%set_env
%make_install

%files
%license LICENSE.md
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*
%exclude %{_cross_mandir}

%files devel
%{_cross_libdir}/*.so
%{_cross_includedir}/*.h
%{_cross_libdir}/*.a
%{_cross_pkgconfigdir}/*.pc
