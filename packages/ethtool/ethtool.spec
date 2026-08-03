Name: %{_cross_os}ethtool
Version: 7.0
Release: 1%{?dist}
Summary: Settings tool for Ethernet NICs
License: GPL-2.0-only AND GPL-2.0-or-later
URL: https://www.kernel.org/pub/software/network/ethtool/
Source0: https://www.kernel.org/pub/software/network/ethtool/ethtool-%{version}.tar.xz
Source1: https://www.kernel.org/pub/software/network/ethtool/ethtool-%{version}.tar.sign
Source2: gpgkey-58DDE3DDB89E566A76EA628EE77F2C1BF2D17695.asc
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libmnl-devel

%description
%{summary}.

%prep
%{gpgverify} --data=<(xzcat %{S:0}) --signature=%{S:1} --keyring=%{S:2}
%setup -n ethtool-%{version}

%build
%cross_configure
%make_build

%install
%make_install

%files
%license COPYING LICENSE
%{_cross_attribution_file}
%{_cross_sbindir}/ethtool
%exclude %{_cross_datadir}/bash-completion
%exclude %{_cross_mandir}
%exclude %{_cross_datadir}/metainfo/org.kernel.software.network.ethtool.metainfo.xml
