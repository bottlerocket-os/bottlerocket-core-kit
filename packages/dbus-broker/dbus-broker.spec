Name: %{_cross_os}dbus-broker
Version: 36
Release: 1%{?dist}
Summary: D-BUS message broker
License: Apache-2.0
URL: https://github.com/bus1/dbus-broker
Source0: https://github.com/bus1/dbus-broker/releases/download/v%{version}/dbus-broker-%{version}.tar.xz
Source1: https://github.com/bus1/dbus-broker/releases/download/v%{version}/dbus-broker-%{version}.tar.xz.asc
Source2: gpgkey-BE5FBC8C9C1C9F60A4F0AEAE7A4F3A09EBDEFF26.asc

Source11: dbus.socket
Source12: dbus-1-system.conf
Source13: dbus-sysusers.conf
Source14: dbus-broker.service
Source15: dbus-broker-launcher.conf

BuildRequires: meson
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libexpat-devel
BuildRequires: %{_cross_os}libselinux-devel
BuildRequires: %{_cross_os}systemd-devel
Requires: %{_cross_os}libexpat
Requires: %{_cross_os}libselinux
Requires: %{_cross_os}systemd
Requires: %{_cross_os}dbus-broker(launcher)

# Work around an aliasing rules violation.
Patch0001: 0001-c-utf8-disable-strict-aliasing-optimizations.patch
# Allow building the journal catalogs when dbus-launcher is excluded
Patch0002: 0002-meson.build-remove-condition-to-build-the-journal-ca.patch

%description
%{summary}.

%package launcher
Summary: A dbus-broker launcher
Provides: %{_cross_os}dbus-broker(launcher) = 1:
Conflicts: %{_cross_os}dbus-broker(launcher)
Requires: %{name}

%description launcher
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n dbus-broker-%{version} -p1

%build
CONFIGURE_OPTS=(
 -Dapparmor=false
 -Daudit=false
 -Ddocs=false
 -Dlauncher=true
 -Dselinux=true
 -Dcatalogdir=%{_cross_journalcatalogdir}
)

%cross_meson "${CONFIGURE_OPTS[@]}"
%cross_meson_build

%install
%cross_meson_install

install -d %{buildroot}%{_cross_unitdir}
install -d %{buildroot}%{_cross_unitdir}/dbus-broker.service.d/
install -p -m 0644 %{S:11} %{S:14} %{buildroot}%{_cross_unitdir}

install -d %{buildroot}%{_cross_datadir}/dbus-1/{interfaces,services,system-services,system.d}
install -p -m 0644 %{S:12} %{buildroot}%{_cross_datadir}/dbus-1/system.conf
install -p -m 0644 %{S:15} %{buildroot}%{_cross_unitdir}/dbus-broker.service.d/

install -d %{buildroot}%{_cross_sysusersdir}
install -p -m 0644 %{S:13} %{buildroot}%{_cross_sysusersdir}/dbus.conf

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_bindir}/dbus-broker
%dir %{_cross_datadir}/dbus-1
%{_cross_journalcatalogdir}/dbus-broker.catalog
%{_cross_sysusersdir}/dbus.conf
%{_cross_unitdir}/dbus.socket
%{_cross_unitdir}/dbus-broker.service
%exclude %{_cross_userunitdir}/dbus-broker.service

%files launcher
%{_cross_bindir}/dbus-broker-launch
%{_cross_unitdir}/dbus-broker.service.d/dbus-broker-launcher.conf
%{_cross_journalcatalogdir}/dbus-broker-launch.catalog
%{_cross_datadir}/dbus-1/*

%changelog
