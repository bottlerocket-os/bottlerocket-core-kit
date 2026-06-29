Name: %{_cross_os}iproute
Version: 7.1.0
Release: 1%{?dist}
Summary: Tools for advanced IP routing and network device configuration
License: GPL-2.0-or-later AND GPL-2.0-only
URL: https://kernel.org/pub/linux/utils/net/iproute2/
Source0: https://kernel.org/pub/linux/utils/net/iproute2/iproute2-%{version}.tar.xz
Source1: https://kernel.org/pub/linux/utils/net/iproute2/iproute2-%{version}.tar.sign
Source2: gpgkey-9F6FC345B05BE7E766B83C8F80A77F6095CDE47E.asc
Patch1: 0001-skip-libelf-check.patch

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libcap-devel
BuildRequires: %{_cross_os}libmnl-devel
BuildRequires: %{_cross_os}libselinux-devel
Requires: %{_cross_os}libcap
Requires: %{_cross_os}libmnl
Requires: %{_cross_os}libselinux

%description
%{summary}.

%prep
%{gpgverify} --data=<(xzcat %{S:0}) --signature=%{S:1} --keyring=%{S:2}
%autosetup -n iproute2-%{version} -p1

%global set_env \
export CC="%{_cross_target}-gcc" \\\
export HOSTCC="gcc" \\\
export DESTDIR='%{buildroot}' \\\
export SBINDIR='%{_cross_sbindir}' \\\
export MANDIR='%{_cross_mandir}' \\\
export LIBDIR='%{_cross_libdir}' \\\
export CONF_USR_DIR='%{_cross_datadir}/iproute2' \\\
export DOCDIR='%{_cross_docdir}' \\\
export HDRDIR='%{_cross_includedir}' \\\
export BASH_COMPDIR='%{_cross_bashdir}' \\\
export PKG_CONFIG_PATH='%{_cross_pkgconfigdir}' \\\
%{nil}

%build
%set_env
%set_cross_build_flags
./configure --libdir '%{_cross_libdir}'
%make_build

%install
%set_env
%make_install

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_sbindir}/bridge
%{_cross_sbindir}/ctstat
%{_cross_sbindir}/dcb
%{_cross_sbindir}/devlink
%{_cross_sbindir}/genl
%{_cross_sbindir}/ifstat
%{_cross_sbindir}/ip
%{_cross_sbindir}/lnstat
%{_cross_sbindir}/nstat
%{_cross_sbindir}/rdma
%{_cross_sbindir}/rtacct
%{_cross_sbindir}/rtmon
%{_cross_sbindir}/rtstat
%{_cross_sbindir}/ss
%{_cross_sbindir}/tc
%{_cross_sbindir}/tipc
%{_cross_sbindir}/vdpa
%{_cross_sbindir}/netshaper
%{_cross_sbindir}/dpll
%dir %{_cross_libdir}/tc
%{_cross_libdir}/tc/*
%dir %{_cross_datadir}/iproute2
%{_cross_datadir}/iproute2/*
%exclude %{_cross_bashdir}/*
%exclude %{_cross_mandir}/*
%exclude %{_cross_includedir}/*
%exclude %{_cross_sbindir}/routel

%changelog
