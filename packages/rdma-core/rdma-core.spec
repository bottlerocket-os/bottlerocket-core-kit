%global abiver rdmav34

Name: %{_cross_os}rdma-core
Version: 54.0
Release: 1%{?dist}
Summary: RDMA core userspace infrastructure, including core libraries and util programs.
License: Linux-OpenIB AND MIT
Source0: https://github.com/linux-rdma/rdma-core/releases/download/v%{version}/rdma-core-%{version}.tar.gz
Source100: libibverbs-tmpfiles.conf

# RDMA logdog configuration
Source200: logdog.rdma.conf

BuildRequires: cmake
BuildRequires: %{_cross_os}libnl-devel
BuildRequires: %{_cross_os}glibc-devel
Requires: %{_cross_os}libnl

%description
%{summary}.

%package devel
Summary: RDMA core development libraries and headers
Requires: %{name}

%description devel
%{summary}.

%prep
%autosetup -n rdma-core-%{version} -p1

%build
%{cross_cmake} . \
  -DNO_PYVERBS=1 \
  -DNO_MAN_PAGES=1 \
  -DCMAKE_BUILD_TYPE=Release \
  -DCMAKE_INSTALL_PREFIX:PATH=%{_cross_prefix} \
  -DCMAKE_INSTALL_BINDIR:PATH=%{_cross_bindir} \
  -DCMAKE_INSTALL_SBINDIR:PATH=%{_cross_sbindir} \
  -DCMAKE_INSTALL_SYSCONFDIR:PATH=%{_cross_sysconfdir} \

%make_build

%install
%make_install

install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m 0644 %{S:100} %{buildroot}%{_cross_tmpfilesdir}/rdma-core.conf

install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/libibverbs.d
install -p %{buildroot}%{_cross_sysconfdir}/libibverbs.d/efa.driver %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/libibverbs.d

install -d %{buildroot}%{_cross_datadir}/logdog.d
install -p -m 0644 %{S:200} %{buildroot}%{_cross_datadir}/logdog.d

%files
%license COPYING.md COPYING.BSD_MIT ccan/LICENSE.MIT
%{_cross_attribution_file}
%{_cross_datadir}/logdog.d/logdog.rdma.conf
%{_cross_tmpfilesdir}/rdma-core.conf
%dir %{_cross_factorydir}%{_cross_sysconfdir}/libibverbs.d
%{_cross_factorydir}%{_cross_sysconfdir}/libibverbs.d/efa.driver

# Core RDMA libraries
%{_cross_libdir}/libibverbs.so.*
%{_cross_libdir}/librdmacm.so.*
%dir %{_cross_libdir}/libibverbs

# EFA libraries
%{_cross_libdir}/libefa.so.*
%{_cross_libdir}/libibverbs/libefa-%{abiver}.so

# Verification tools
%{_cross_bindir}/ibv_devices
%{_cross_bindir}/ibv_devinfo

# Exclude the bits that are not needed
%exclude %{_cross_datadir}/perl5
%exclude %{_cross_docdir}
%exclude %{_cross_libdir}/udev
%exclude %{_cross_libexecdir}
%exclude %{_cross_pkgconfigdir}
%exclude %{_cross_sbindir}
%exclude %{_cross_sysconfdir}
%exclude %{_cross_unitdir}

# Exclude all the unused libs
%exclude %{_cross_libdir}/ibacm*
%exclude %{_cross_libdir}/libbnxt*
%exclude %{_cross_libdir}/libcxgb4*
%exclude %{_cross_libdir}/liberdma*
%exclude %{_cross_libdir}/libhfi1*
%exclude %{_cross_libdir}/libhns*
%exclude %{_cross_libdir}/libibmad*
%exclude %{_cross_libdir}/libibnetdisc*
%exclude %{_cross_libdir}/libibumad*
%exclude %{_cross_libdir}/libmana*
%exclude %{_cross_libdir}/libmlx*
%exclude %{_cross_libdir}/libmthca*
%exclude %{_cross_libdir}/libocrdma*
%exclude %{_cross_libdir}/libqedr*
%exclude %{_cross_libdir}/librxe*
%exclude %{_cross_libdir}/libsiw*
%exclude %{_cross_libdir}/libvmw*
%exclude %{_cross_libdir}/rsocket

# Exclude specific RDMA providers (keeping only libefa)
%exclude %{_cross_libdir}/libibverbs/libbnxt_re-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libcxgb4-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/liberdma-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libhfi1verbs-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libhns-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libipathverbs-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libirdma-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libmana-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libmlx4-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libmlx5-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libmthca-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libocrdma-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libqedr-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/librxe-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libsiw-%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libvmw_pvrdma-%{abiver}.so

# Exclude udev rules
%exclude %{_cross_udevrulesdir}

# Exclude all the unused binaries
%exclude %{_cross_bindir}/cmtime
%exclude %{_cross_bindir}/ib_acme
%exclude %{_cross_bindir}/ibv_asyncwatch
%exclude %{_cross_bindir}/ibv_rc_pingpong
%exclude %{_cross_bindir}/ibv_srq_pingpong
%exclude %{_cross_bindir}/ibv_uc_pingpong
%exclude %{_cross_bindir}/ibv_ud_pingpong
%exclude %{_cross_bindir}/ibv_xsrq_pingpong
%exclude %{_cross_bindir}/mckey
%exclude %{_cross_bindir}/rcopy
%exclude %{_cross_bindir}/rdma_client
%exclude %{_cross_bindir}/rdma_server
%exclude %{_cross_bindir}/rdma_xclient
%exclude %{_cross_bindir}/rdma_xserver
%exclude %{_cross_bindir}/riostream
%exclude %{_cross_bindir}/rping
%exclude %{_cross_bindir}/rstream
%exclude %{_cross_bindir}/ucmatose
%exclude %{_cross_bindir}/udaddy
%exclude %{_cross_bindir}/udpong

%files devel
%dir %{_cross_includedir}/infiniband
%dir %{_cross_includedir}/rdma
%{_cross_includedir}/infiniband/*
%{_cross_includedir}/rdma/*
%{_cross_libdir}/libefa*
%{_cross_libdir}/libibverbs*
%{_cross_libdir}/librdmacm*

%changelog
