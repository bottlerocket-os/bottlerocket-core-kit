Name: %{_cross_os}libisal
Version: 2.32.0
Release: 1%{?dist}
Summary: Library for Intel ISA
License: BSD-3-Clause
URL: https://github.com/intel/isa-l
Source0: https://github.com/intel/isa-l/archive/v%{version}/isa-l-%{version}.tar.gz

# Patch to optimize stdin / stdout pipe usage for containerd.
Patch1001: 1001-igzip-increase-stdin-and-stdout-pipe-sizes.patch

BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for Intel ISA
Requires: %{name}

%description devel
%{summary}.

%package -n %{_cross_os}igzip
Summary: Compress or decompress files using the library for Intel ISA
Requires: %{name}

%description -n %{_cross_os}igzip
%{summary}.

%prep
%autosetup -n isa-l-%{version} -p1

%build
autoreconf -fi
%cross_configure \
  --enable-static \
  %{nil}

%force_disable_rpath

%make_build

%install
%make_install

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*
%exclude %{_cross_mandir}

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.so
%{_cross_includedir}/isa-l.h
%dir %{_cross_includedir}/isa-l
%{_cross_includedir}/isa-l/*.h
%{_cross_pkgconfigdir}/*.pc

%files -n %{_cross_os}igzip
%{_cross_bindir}/igzip

%changelog
