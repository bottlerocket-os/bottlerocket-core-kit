Name: %{_cross_os}iptables
Version: 1.8.11
Release: 1%{?dist}
Epoch: 1
Summary: Tools for managing Linux kernel packet filtering capabilities
License: GPL-2.0-or-later AND GPL-2.0-only
URL: http://www.netfilter.org/
Source0: https://www.netfilter.org/projects/iptables/files/iptables-%{version}.tar.xz
Source1: https://www.netfilter.org/projects/iptables/files/iptables-%{version}.tar.xz.sig
Source2: gpgkey-8C5F7146A1757A65E2422A94D70D1A666ACF2B21.asc

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libmnl-devel
BuildRequires: %{_cross_os}libnfnetlink-devel
BuildRequires: %{_cross_os}libnftnl-devel
BuildRequires: %{_cross_os}libnetfilter_conntrack-devel
Requires: %{_cross_os}libmnl
Requires: %{_cross_os}libnfnetlink
Requires: %{_cross_os}libnftnl
Requires: %{_cross_os}libnetfilter_conntrack

%description
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
  --disable-bpf-compiler \
  --disable-connlabel \
  --disable-libipq \
  --disable-nftables \
  --disable-nfsynproxy \
  --disable-static \

%force_disable_rpath

%make_build

%cross_generate_sbom

%install
%make_install

# Replace absolute symlink with relative one.
ln -srnf \
  %{buildroot}%{_cross_sbindir}/xtables-legacy-multi \
  %{buildroot}%{_cross_bindir}/iptables-xml

%cross_install_sbom

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%{_cross_sbindir}/xtables-legacy-multi
%{_cross_sbindir}/iptables
%{_cross_sbindir}/iptables-legacy
%{_cross_sbindir}/iptables-legacy-restore
%{_cross_sbindir}/iptables-legacy-save
%{_cross_sbindir}/iptables-restore
%{_cross_sbindir}/iptables-save
%{_cross_sbindir}/ip6tables
%{_cross_sbindir}/ip6tables-legacy
%{_cross_sbindir}/ip6tables-legacy-restore
%{_cross_sbindir}/ip6tables-legacy-save
%{_cross_sbindir}/ip6tables-restore
%{_cross_sbindir}/ip6tables-save
%{_cross_bindir}/iptables-xml
%{_cross_libdir}/*.so.*
%dir %{_cross_libdir}/xtables
%{_cross_libdir}/xtables/*.so
%exclude %{_cross_mandir}/*
%exclude %{_cross_datadir}/xtables/pf.os
%exclude %{_cross_datadir}/xtables/iptables.xslt
%exclude %{_cross_sbindir}/iptables-apply
%exclude %{_cross_sbindir}/ip6tables-apply
%exclude %{_cross_sbindir}/nfnl_osf

%files devel
%{_cross_libdir}/*.so
%{_cross_includedir}/*.h
%dir %{_cross_includedir}/libiptc
%{_cross_includedir}/libiptc/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
