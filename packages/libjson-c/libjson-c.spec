%global jsonc_ver 0.18
%global jsonc_rel 20240915

Name: %{_cross_os}libjson-c
Version: %{jsonc_ver}
Release: %{jsonc_rel}%{?dist}
Summary: Library for JSON
License: MIT
Source0: https://github.com/json-c/json-c/archive/refs/tags/json-c-%{jsonc_ver}-%{jsonc_rel}.tar.gz
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for JSON
Requires: %{name}

%description devel
%{summary}.

%prep
%autosetup -n json-c-json-c-%{jsonc_ver}-%{jsonc_rel} -p1

%build
%{cross_cmake} . \
  -DBUILD_APPS:BOOL=OFF \
  -DBUILD_STATIC_LIBS:BOOL=OFF \
  -DDISABLE_BSYMBOLIC:BOOL=OFF \
  -DDISABLE_EXTRA_LIBS:BOOL=ON \
  -DDISABLE_JSON_POINTER:BOOL=ON \
  -DDISABLE_THREAD_LOCAL_STORAGE:BOOL=OFF \
  -DDISABLE_WERROR:BOOL=OFF \
  -DENABLE_RDRAND:BOOL=ON \
  -DENABLE_THREADING:BOOL=ON \
  -DCMAKE_INSTALL_PREFIX:PATH=%{_cross_prefix} \
  -G Ninja

cmake --build .

%install
DESTDIR="%{buildroot}" cmake --install .

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*
%exclude %{_cross_libdir}/cmake

%files devel
%{_cross_libdir}/*.so
%dir %{_cross_includedir}/json-c
%{_cross_includedir}/json-c/*.h
%{_cross_pkgconfigdir}/*.pc
