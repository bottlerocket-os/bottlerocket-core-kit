Name: %{_cross_os}libjansson
Version: 2.15.1
Release: 1%{?dist}
Summary: Library for encoding, decoding and manipulating JSON data
License: MIT
URL: https://github.com/akheron/jansson
Source0: https://github.com/akheron/jansson/archive/v%{version}.tar.gz#/jansson-%{version}.tar.gz
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for encoding, decoding and manipulating JSON data
Requires: %{name}

%description devel
%{summary}.

%prep
%autosetup -n jansson-%{version} -p1

%build
autoreconf -fiv

%cross_configure \
  --disable-static \
  --disable-dtoa \
  %{nil}

%make_build

%install
%make_install

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*

%files devel
%{_cross_includedir}/*.h
%{_cross_libdir}/*.so
%{_cross_pkgconfigdir}/*.pc
