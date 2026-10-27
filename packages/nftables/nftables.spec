Name: %{_cross_os}nftables
Version: 1.1.7
Release: 1%{?dist}
Summary: Tools for managing Netfilter tables
License: GPL-2.0-only
URL: http://netfilter.org/projects/nftables
Source0: http://netfilter.org/projects/nftables/files/nftables-%{version}.tar.xz
Source1: http://netfilter.org/projects/nftables/files/nftables-%{version}.tar.xz.sig
Source2: gpgkey-8C5F7146A1757A65E2422A94D70D1A666ACF2B21.asc
Source10: nftables-tmpfiles.conf

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}iptables-devel
BuildRequires: %{_cross_os}libjansson-devel
BuildRequires: %{_cross_os}libmnl-devel
BuildRequires: %{_cross_os}libnftnl-devel
BuildRequires: %{_cross_os}readline-devel
Requires: %{_cross_os}iptables
Requires: %{_cross_os}libjansson
Requires: %{_cross_os}libmnl
Requires: %{_cross_os}libnftnl
Requires: %{_cross_os}readline

%description
%{summary}.

%package devel
Summary: Files for development using the tools for managing Netfilter tables
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n nftables-%{version} -p1

%build
%cross_configure \
  --disable-man-doc \
  --enable-debug \
  --with-cli=readline \
  --with-mini-gmp \
  --with-json \
  --with-xtables \
  %{nil}

%force_disable_rpath

%make_build

%install
%make_install

install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/nftables/osf
mv %{buildroot}{,%{_cross_factorydir}}%{_cross_sysconfdir}/nftables/osf/pf.os

install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m 0644 %{S:10} %{buildroot}%{_cross_tmpfilesdir}/nftables.conf

find %{buildroot} -name '*.nft' -delete

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_sbindir}/nft
%{_cross_libdir}/*.so.*
%{_cross_tmpfilesdir}/nftables.conf
%dir %{_cross_factorydir}%{_cross_sysconfdir}/nftables
%{_cross_factorydir}%{_cross_sysconfdir}/nftables/osf/pf.os

%files devel
%{_cross_libdir}/*.so
%dir %{_cross_includedir}/nftables
%{_cross_includedir}/nftables/*.h
%{_cross_pkgconfigdir}/*.pc
