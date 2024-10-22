Name: %{_cross_os}libbpf
Version: 1.4.5
Release: 1%{?dist}
Epoch: 1
Summary: Library for BPF
License: LGPL-2.1-only OR BSD-2-Clause
URL: https://github.com/libbpf/libbpf
Source0: https://github.com/libbpf/libbpf/archive/refs/tags/v%{version}.tar.gz
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libelf-devel
BuildRequires: %{_cross_os}libz-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for BPF
Requires: %{name}

%description devel
%{summary}.

%prep
%autosetup -n libbpf-%{version} -p1

%global kmake \
make -s \\\
  CROSS_COMPILE="%{_cross_target}-" \\\
  DESTDIR="%{buildroot}" \\\
  PREFIX="%{_cross_prefix}" \\\
  LIBDIR="%{_cross_libdir}" \\\
  INCLUDEDIR="%{_cross_includedir}" \\\
%{nil}

%build
%kmake -C src

%install
%kmake -C src install

%files
%license LICENSE LICENSE.BSD-2-Clause LICENSE.LGPL-2.1
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.so
%dir %{_cross_includedir}/bpf
%{_cross_includedir}/bpf/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
