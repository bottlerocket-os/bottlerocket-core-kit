Name: %{_cross_os}hwloc
Version: 2.12.2
Release: 1%{?dist}
Summary: Portable hardware locality library
URL: https://www.open-mpi.org/projects/hwloc/
License: BSD-3-Clause
Source0: https://download.open-mpi.org/release/hwloc/v2.12/hwloc-%{version}.tar.bz2

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}systemd-devel

%description
%{summary}.

%package tools
Summary: Command line tools for the hwloc library
Requires: %{name}

%description tools
%{summary}.

%package devel
Summary: hwloc development libraries and headers
Requires: %{name}
Requires: %{_cross_os}systemd-devel

%description devel
%{summary}.

%prep
%autosetup -n hwloc-%{version} -p1

%build
%cross_configure \
    --disable-cairo \
    --disable-gl \
    --disable-libxml2 \
    --disable-opencl \
    --disable-pci \
    --disable-plugins \
    --exec-prefix=%{_cross_prefix} \
    --program-prefix=""

%force_disable_rpath

%make_build

%install
%make_install

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_libdir}/libhwloc.so
%{_cross_libdir}/libhwloc.so.*
%exclude %{_cross_bindir}/lstopo
%exclude %{_cross_bindir}/hwloc-compress-dir
%exclude %{_cross_bindir}/hwloc-gather-topology
%exclude %{_cross_datadir}
%exclude %{_cross_mandir}

%files tools
%{_cross_bindir}/hwloc-annotate
%{_cross_bindir}/hwloc-ls
%{_cross_bindir}/hwloc-ps
%{_cross_bindir}/hwloc-bind
%{_cross_bindir}/hwloc-calc
%{_cross_bindir}/hwloc-diff
%{_cross_bindir}/hwloc-distrib
%{_cross_bindir}/hwloc-info
%{_cross_bindir}/hwloc-patch
%{_cross_bindir}/lstopo-no-graphics
# These are not on aarch64
%if "%{_cross_arch}" == "x86_64"
%{_cross_sbindir}/hwloc-dump-hwdata
%{_cross_bindir}/hwloc-gather-cpuid
%endif

%files devel
%{_cross_includedir}/hwloc.h
%{_cross_includedir}/hwloc/*.h
%{_cross_includedir}/hwloc/autogen/*.h
%{_cross_pkgconfigdir}/*.pc
