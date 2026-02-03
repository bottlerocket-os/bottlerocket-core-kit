%global goproject github.com/notaryproject
%global gorepo notation
%global goimport %{goproject}/%{gorepo}

%global gover 1.3.2
%global rpmver %{gover}
%global gitrev 001cc919603c1dc16c6aad387c94b4209cb9c901

%global _dwz_low_mem_die_limit 0

%global notation_configdir %{_cross_sysconfdir}/containerd/image-verifiers/notation

Name: %{_cross_os}%{gorepo}
Version: %{rpmver}
Release: 1%{?dist}
Summary: A CLI tool to sign and verify artifacts.
License: Apache-2.0
URL: https://%{goimport}
Source0: https://%{goimport}/archive/v%{gover}/%{gorepo}-v%{gover}.tar.gz
Source1: bundled-%{gorepo}-v%{gover}.tar.gz
Source2: notation-trust-policy-json
Source3: notation-tmpfiles.conf

BuildRequires: %{_cross_os}glibc-devel
Requires: %{_cross_os}ecr-credential-helper

%description
%{summary}.

%prep
%setup -n %{gorepo}-%{gover} -q
%setup -T -D -n %{gorepo}-%{gover} -b 1 -q

%build
%set_cross_go_flags

go build -ldflags "${GOLDFLAGS}" -o notation ./cmd/notation

%install
install -d %{buildroot}%{_cross_bindir}
install -d %{buildroot}%{_cross_templatedir}

install -p -m 0755 notation %{buildroot}%{_cross_bindir}
install -p -m 0644 %{S:2} %{buildroot}%{_cross_templatedir}/notation-trust-policy-json

# Add the notation config and cache directories
install -d %{buildroot}%{_cross_factorydir}%{notation_configdir}/plugins
install -d %{buildroot}%{_cross_factorydir}%{notation_configdir}/truststore/x509/signingAuthority

install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m 0644 %{S:3} %{buildroot}%{_cross_tmpfilesdir}/notation.conf

%cross_scan_attribution go-vendor vendor

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%{_cross_bindir}/notation
%{_cross_templatedir}/notation-trust-policy-json
%{_cross_tmpfilesdir}/notation.conf
%dir %{_cross_factorydir}%{notation_configdir}
%dir %{_cross_factorydir}%{notation_configdir}/plugins
%dir %{_cross_factorydir}%{notation_configdir}/truststore/x509/signingAuthority

%changelog
