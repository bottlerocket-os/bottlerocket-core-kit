Name: %{_cross_os}chrony
Version: 4.6.1
Release: 1%{?dist}
Summary: A versatile implementation of the Network Time Protocol
License: GPL-2.0-only
URL: https://chrony.tuxfamily.org
Source0: https://download.tuxfamily.org/chrony/chrony-%{version}.tar.gz
Source1: https://chrony-project.org/releases/chrony-%{version}-tar-gz-asc.txt
Source2: gpgkey-8F375C7E8D0EE125A3D3BD51537E2B76F7680DAC.asc

Source11: chronyd.service
Source12: chrony-conf
Source13: chrony-sysusers.conf
Source14: chrony-tmpfiles.conf

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libcap-devel
BuildRequires: %{_cross_os}libncurses-devel
BuildRequires: %{_cross_os}libseccomp-devel
BuildRequires: %{_cross_os}readline-devel
Requires: %{_cross_os}libcap
Requires: %{_cross_os}libseccomp

%description
%{summary}.

%package tools
Summary: Command-line interface for chrony daemon
Requires: %{_cross_os}chrony
Requires: %{_cross_os}libncurses
Requires: %{_cross_os}readline

%description tools
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n chrony-%{version} -p1

%build
# chrony uses a custom hand-rolled configure script
%set_cross_build_flags \
CC=%{_cross_target}-gcc \
./configure \
 --prefix="%{_cross_prefix}" \
 --enable-scfilter

%make_build
%cross_generate_sbom

%install
%make_install

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:11} %{buildroot}%{_cross_unitdir}/chronyd.service
install -d %{buildroot}%{_cross_templatedir}
install -p -m 0644 %{S:12} %{buildroot}%{_cross_templatedir}/chrony-conf
install -d %{buildroot}%{_cross_sysusersdir}
install -p -m 0644 %{S:13} %{buildroot}%{_cross_sysusersdir}/chrony.conf
install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m 0644 %{S:14} %{buildroot}%{_cross_tmpfilesdir}/chrony.conf

%cross_install_sbom

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%dir %{_cross_templatedir}
%{_cross_sbindir}/chronyd
%{_cross_templatedir}/chrony-conf
%{_cross_unitdir}/chronyd.service
%{_cross_sysusersdir}/chrony.conf
%{_cross_tmpfilesdir}/chrony.conf
%exclude %{_cross_mandir}

%files tools
%{_cross_bindir}/chronyc

%changelog
