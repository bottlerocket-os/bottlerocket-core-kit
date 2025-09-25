%global _cross_first_party 1
%undefine _debugsource_packages

Name: %{_cross_os}netdog
Version: 0.1.1
Release: 1%{?dist}
Epoch: 1
Summary: Bottlerocket network configuration helper
License: Apache-2.0 OR MIT
URL: https://github.com/bottlerocket-os/bottlerocket

Source10: run-netdog.mount
Source11: write-network-status.service
Source12: generate-network-config.service
Source13: disable-udp-offload.service

Source20: 00-resolved.conf

BuildRequires: %{_cross_os}glibc-devel
Requires: %{_cross_os}hostname-reverse-dns
Requires: (%{_cross_os}hostname-imds if %{_cross_os}variant-platform(aws))
Requires: %{_cross_os}systemd-networkd
Requires: %{_cross_os}systemd-resolved

%description
%{summary}.

%package -n %{_cross_os}hostname-reverse-dns
Summary: Reverse DNS Hostname detector
%description -n %{_cross_os}hostname-reverse-dns
%{summary}

%package -n %{_cross_os}hostname-imds
Summary: IMDS Hostname detector
%description -n %{_cross_os}hostname-imds
%{summary}

%prep
%setup -T -c
%cargo_prep

%build
mkdir bin

echo "** Build Dogtag Hostname Detectors"
%cargo_build --manifest-path %{_builddir}/sources/Cargo.toml \
    -p dogtag \
    --bins \
    --target-dir=${HOME}/.cache/dogtag

echo "** Build Netdog Binaries"
%cargo_build --manifest-path %{_builddir}/sources/Cargo.toml \
    -p netdog \
    --features default \
    --target-dir=${HOME}/.cache/networkd

%install
install -d %{buildroot}%{_cross_libexecdir}/hostname-detectors
install -p -m 0755 ${HOME}/.cache/dogtag/%{__cargo_target}/release/20-imds %{buildroot}%{_cross_libexecdir}/hostname-detectors/20-imds
install -p -m 0755 ${HOME}/.cache/dogtag/%{__cargo_target}/release/10-reverse-dns %{buildroot}%{_cross_libexecdir}/hostname-detectors/10-reverse-dns

install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 ${HOME}/.cache/networkd/%{__cargo_target}/release/netdog %{buildroot}%{_cross_bindir}/netdog

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:10} %{S:11} %{S:12} %{S:13} %{buildroot}%{_cross_unitdir}

install -d %{buildroot}%{_cross_libdir}
install -d %{buildroot}%{_cross_libdir}/systemd/resolved.conf.d
install -p -m 0644 %{S:20} %{buildroot}%{_cross_libdir}/systemd/resolved.conf.d

%files
%{_cross_unitdir}/generate-network-config.service
%{_cross_unitdir}/disable-udp-offload.service
%{_cross_unitdir}/run-netdog.mount
%{_cross_bindir}/netdog
%{_cross_unitdir}/write-network-status.service
%dir %{_cross_libdir}/systemd/resolved.conf.d
%{_cross_libdir}/systemd/resolved.conf.d/00-resolved.conf

%files -n %{_cross_os}hostname-reverse-dns
%{_cross_libexecdir}/hostname-detectors/10-reverse-dns

%files -n %{_cross_os}hostname-imds
%{_cross_libexecdir}/hostname-detectors/20-imds
