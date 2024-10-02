%global _cross_first_party 1

Name: %{_cross_os}login
Version: 0.0.1
Release: 1%{?dist}
Summary: A login helper
License: Apache-2.0 OR MIT
URL: https://github.com/bottlerocket-os/bottlerocket
BuildRequires: %{_cross_os}glibc-devel
Requires: %{_cross_os}bash
Requires: %{_cross_os}systemd-console

Source0: login
Source1: getty.drop-in.conf

%description
%{summary}.

%prep

%build

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 %{S:0} %{buildroot}%{_cross_bindir}/login

install -d %{buildroot}%{_cross_sbindir}
ln -s ../bin/login %{buildroot}%{_cross_sbindir}/sulogin

install -d %{buildroot}%{_cross_unitdir}/getty.target.d
install -p -m 0644 %{S:1} %{buildroot}%{_cross_unitdir}/getty.target.d/001-login.conf

%files
%{_cross_bindir}/login
%{_cross_sbindir}/sulogin
%{_cross_unitdir}/getty.target.d/001-login.conf

%changelog
