%global _cross_first_party 1
%undefine _debugsource_packages

Name: %{_cross_os}rottweiler
Version: 0.1.0
Release: 1%{?dist}
Summary: Bottlerocket storage encryption helper
License: Apache-2.0 OR MIT
URL: https://github.com/bottlerocket-os/bottlerocket

BuildRequires: %{_cross_os}glibc-devel
Requires: %{_cross_os}cryptsetup
Requires: %{_cross_os}systemd-cryptsetup
Requires: %{_cross_os}tpm2-tools

%description
%{summary}.

%prep
%setup -T -c
%cargo_prep

%build
%cargo_build --manifest-path %{_builddir}/sources/Cargo.toml \
    -p rottweiler

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 %{__cargo_outdir}/rottweiler %{buildroot}%{_cross_bindir}
ln -s rottweiler %{buildroot}%{_cross_bindir}/rw

%files
%{_cross_bindir}/rottweiler
%{_cross_bindir}/rw
