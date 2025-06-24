%global abiver 57

Name: %{_cross_os}rdma-core
Version: 57.0
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
  -DCMAKE_INSTALL_UDEV_RULESDIR:PATH=%{_cross_udevrulesdir} \

%make_build

%cross_generate_sbom

%install
%make_install

install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m 0644 %{S:100} %{buildroot}%{_cross_tmpfilesdir}/rdma-core.conf

install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/libibverbs.d
install -p %{buildroot}%{_cross_sysconfdir}/libibverbs.d/efa.driver %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/libibverbs.d

install -d %{buildroot}%{_cross_datadir}/logdog.d
install -p -m 0644 %{S:200} %{buildroot}%{_cross_datadir}/logdog.d

%cross_install_sbom

%files
%license COPYING.md COPYING.BSD_MIT ccan/LICENSE.MIT
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%{_cross_datadir}/logdog.d/logdog.rdma.conf
%{_cross_tmpfilesdir}/rdma-core.conf
%dir %{_cross_factorydir}%{_cross_sysconfdir}/libibverbs.d
%{_cross_factorydir}%{_cross_sysconfdir}/libibverbs.d/efa.driver

# Core RDMA libraries
%{_cross_libdir}/libibmad.so.*
%{_cross_libdir}/libibumad.so.*
%{_cross_libdir}/libibnetdisc.so.*
%{_cross_libdir}/libibverbs.so.*
%{_cross_libdir}/librdmacm.so.*
%dir %{_cross_libdir}/libibverbs

# EFA libraries
%{_cross_libdir}/libefa.so.*
%{_cross_libdir}/libibverbs/libefa-rdmav%{abiver}.so

# Verification tools
%{_cross_bindir}/ibv_devices
%{_cross_bindir}/ibv_devinfo
%{_cross_sbindir}/ibstat

# udev rule for renaming to persistent names
%{_cross_libdir}/udev/rdma_rename
%{_cross_udevrulesdir}/60-rdma-persistent-naming.rules
%{_cross_udevrulesdir}/90-rdma-umad.rules

# Exclude the other udev rules we don't want
%exclude %{_cross_udevrulesdir}/60-srp_daemon.rules
%exclude %{_cross_udevrulesdir}/75-rdma-description.rules
%exclude %{_cross_udevrulesdir}/90-iwpmd.rules
%exclude %{_cross_udevrulesdir}/90-rdma-hw-modules.rules
%exclude %{_cross_udevrulesdir}/90-rdma-ulp-modules.rules

# Exclude the bits that are not needed
%exclude %{_cross_datadir}/perl5
%exclude %{_cross_docdir}
%exclude %{_cross_libexecdir}
%exclude %{_cross_pkgconfigdir}
%exclude %{_cross_sysconfdir}
%exclude %{_cross_unitdir}

# Exclude all the unused libs
%exclude %{_cross_libdir}/ibacm*
%exclude %{_cross_libdir}/libhns*
%exclude %{_cross_libdir}/libmana*
%exclude %{_cross_libdir}/libmlx*
%exclude %{_cross_libdir}/rsocket

# Exclude specific RDMA providers (keeping only libefa)
%exclude %{_cross_libdir}/libibverbs/libbnxt_re-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libcxgb4-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/liberdma-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libhfi1verbs-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libhns-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libipathverbs-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libirdma-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libmana-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libmlx4-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libmlx5-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libmthca-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libocrdma-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libqedr-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/librxe-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libsiw-rdmav%{abiver}.so
%exclude %{_cross_libdir}/libibverbs/libvmw_pvrdma-rdmav%{abiver}.so

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
%exclude %{_cross_sbindir}/check_lft_balance.pl
%exclude %{_cross_sbindir}/dump_fts
%exclude %{_cross_sbindir}/dump_lfts.sh
%exclude %{_cross_sbindir}/dump_mfts.sh
%exclude %{_cross_sbindir}/ibacm
%exclude %{_cross_sbindir}/ibaddr
%exclude %{_cross_sbindir}/ibcacheedit
%exclude %{_cross_sbindir}/ibccconfig
%exclude %{_cross_sbindir}/ibccquery
%exclude %{_cross_sbindir}/ibfindnodesusing.pl
%exclude %{_cross_sbindir}/ibhosts
%exclude %{_cross_sbindir}/ibidsverify.pl
%exclude %{_cross_sbindir}/iblinkinfo
%exclude %{_cross_sbindir}/ibnetdiscover
%exclude %{_cross_sbindir}/ibnodes
%exclude %{_cross_sbindir}/ibping
%exclude %{_cross_sbindir}/ibportstate
%exclude %{_cross_sbindir}/ibqueryerrors
%exclude %{_cross_sbindir}/ibroute
%exclude %{_cross_sbindir}/ibrouters
%exclude %{_cross_sbindir}/ibsrpdm
%exclude %{_cross_sbindir}/ibstatus
%exclude %{_cross_sbindir}/ibswitches
%exclude %{_cross_sbindir}/ibsysstat
%exclude %{_cross_sbindir}/ibtracert
%exclude %{_cross_sbindir}/iwpmd
%exclude %{_cross_sbindir}/perfquery
%exclude %{_cross_sbindir}/run_srp_daemon
%exclude %{_cross_sbindir}/saquery
%exclude %{_cross_sbindir}/sminfo
%exclude %{_cross_sbindir}/smpdump
%exclude %{_cross_sbindir}/smpquery
%exclude %{_cross_sbindir}/srp_daemon
%exclude %{_cross_sbindir}/srp_daemon.sh
%exclude %{_cross_sbindir}/vendstat

%files devel
%dir %{_cross_includedir}/infiniband
%dir %{_cross_includedir}/rdma
%{_cross_includedir}/infiniband/*
%{_cross_includedir}/rdma/*
%{_cross_libdir}/libefa.so
%{_cross_libdir}/libibmad.so
%{_cross_libdir}/libibnetdisc.so
%{_cross_libdir}/libibumad.so
%{_cross_libdir}/libibverbs.so
%{_cross_libdir}/librdmacm.so

%changelog
