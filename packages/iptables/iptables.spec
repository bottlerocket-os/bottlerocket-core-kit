Name: %{_cross_os}iptables
Version: 1.8.13
Release: 1%{?dist}
Epoch: 1
Summary: Tools for managing Linux kernel packet filtering capabilities
License: GPL-2.0-or-later AND GPL-2.0-only
URL: http://www.netfilter.org/
Source0: https://www.netfilter.org/projects/iptables/files/iptables-%{version}.tar.xz
Source1: https://www.netfilter.org/projects/iptables/files/iptables-%{version}.tar.xz.sig
Source2: gpgkey-8C5F7146A1757A65E2422A94D70D1A666ACF2B21.asc
Source10: iptables-tmpfiles.conf

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libmnl-devel
BuildRequires: %{_cross_os}libnfnetlink-devel
BuildRequires: %{_cross_os}libnftnl-devel
BuildRequires: %{_cross_os}libnetfilter_conntrack-devel
Requires: %{_cross_os}libmnl
Requires: %{_cross_os}libnfnetlink
Requires: %{_cross_os}libnftnl
Requires: %{_cross_os}libnetfilter_conntrack
Requires: %{name}(kernel-api)

%description
%{summary}.

%package legacy
Summary: Tools for managing Linux kernel packet filtering capabilities using the legacy kernel API
Requires: %{name}
Provides: %{name}(kernel-api) = 1:
Conflicts: %{name}-nft
Conflicts: %{_cross_os}nftables

%description legacy
%{summary}.

%package nft
Summary: Tools for managing Linux kernel packet filtering capabilities using the nftables kernel API
Requires: %{name}
Provides: %{name}(kernel-api) = 0:
Conflicts: %{name}-legacy

%description nft
%{summary}.

%package devel
Summary: Files for development using the tools for managing Linux kernel packet filtering capabilities
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n iptables-%{version} -p1

%build
%cross_configure \
  --with-kernel=%{_cross_prefix} \
  --with-kbuild=%{_cross_prefix} \
  --with-ksource=%{_cross_prefix} \
  --enable-nftables \
  --disable-bpf-compiler \
  --disable-connlabel \
  --disable-libipq \
  --disable-nfsynproxy \
  --disable-static \

%force_disable_rpath

%make_build

%install
%make_install

install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}
mv %{buildroot}{,%{_cross_factorydir}}%{_cross_sysconfdir}/ethertypes

install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m 0644 %{S:10} %{buildroot}%{_cross_tmpfilesdir}/iptables.conf

for link in ip{,6}tables{,-restore,-save} ; do
  ln -snf xtables %{buildroot}%{_cross_sbindir}/${link}
done

%post legacy -p <lua>
posix.symlink("xtables-legacy-multi", "%{_cross_sbindir}/xtables")

%post nft -p <lua>
posix.symlink("xtables-nft-multi", "%{_cross_sbindir}/xtables")

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_sbindir}/iptables
%{_cross_sbindir}/iptables-restore
%{_cross_sbindir}/iptables-save
%{_cross_sbindir}/ip6tables
%{_cross_sbindir}/ip6tables-restore
%{_cross_sbindir}/ip6tables-save
%{_cross_libdir}/*.so.*
%dir %{_cross_libdir}/xtables
%{_cross_libdir}/xtables/*.so
%{_cross_tmpfilesdir}/iptables.conf
%{_cross_factorydir}%{_cross_sysconfdir}/ethertypes
%exclude %{_cross_mandir}/*
%exclude %{_cross_datadir}/xtables/pf.os
%exclude %{_cross_datadir}/xtables/iptables.xslt
%exclude %{_cross_bindir}/iptables-xml
%exclude %{_cross_sbindir}/iptables-apply
%exclude %{_cross_sbindir}/ip6tables-apply
%exclude %{_cross_sbindir}/nfnl_osf

%files legacy
%{_cross_sbindir}/xtables-legacy-multi
%{_cross_sbindir}/iptables-legacy
%{_cross_sbindir}/iptables-legacy-restore
%{_cross_sbindir}/iptables-legacy-save
%{_cross_sbindir}/ip6tables-legacy
%{_cross_sbindir}/ip6tables-legacy-restore
%{_cross_sbindir}/ip6tables-legacy-save

%files nft
%{_cross_sbindir}/xtables-nft-multi
%{_cross_sbindir}/xtables-monitor
%{_cross_sbindir}/arptables
%{_cross_sbindir}/arptables-restore
%{_cross_sbindir}/arptables-save
%{_cross_sbindir}/arptables-nft
%{_cross_sbindir}/arptables-nft-restore
%{_cross_sbindir}/arptables-nft-save
%{_cross_sbindir}/ebtables
%{_cross_sbindir}/ebtables-restore
%{_cross_sbindir}/ebtables-save
%{_cross_sbindir}/ebtables-nft
%{_cross_sbindir}/ebtables-nft-restore
%{_cross_sbindir}/ebtables-nft-save
%{_cross_sbindir}/iptables-nft
%{_cross_sbindir}/iptables-nft-restore
%{_cross_sbindir}/iptables-nft-save
%{_cross_sbindir}/ip6tables-nft
%{_cross_sbindir}/ip6tables-nft-restore
%{_cross_sbindir}/ip6tables-nft-save

# Exclude translate helpers since they aren't needed at runtime.
%exclude %{_cross_sbindir}/arptables-translate
%exclude %{_cross_sbindir}/ebtables-translate
%exclude %{_cross_sbindir}/iptables-restore-translate
%exclude %{_cross_sbindir}/iptables-translate
%exclude %{_cross_sbindir}/ip6tables-restore-translate
%exclude %{_cross_sbindir}/ip6tables-translate

%files devel
%{_cross_libdir}/*.so
%{_cross_includedir}/*.h
%dir %{_cross_includedir}/libiptc
%{_cross_includedir}/libiptc/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
