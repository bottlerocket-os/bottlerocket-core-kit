%global goproject github.com/aws
%global gorepo aws-signer-notation-plugin
%global goimport %{goproject}/%{gorepo}

%global gover 1.0.2292
%global rpmver %{gover}

%global _dwz_low_mem_die_limit 0

%global plugin_name com.amazonaws.signer.notation.plugin
%global signing_authority_path notation/truststore/x509/signingAuthority

Name: %{_cross_os}aws-signer-notation-plugin
Version: %{rpmver}
Release: 1%{?dist}
Summary: AWS Signer plugin for Notation
License: Apache-2.0
URL: https://%{goimport}
Source0: https://%{goimport}/archive/v%{gover}/%{gorepo}-v%{gover}.tar.gz
Source1: bundled-%{gorepo}-v%{gover}.tar.gz

# The commercial and gov root certificates for AWS Signer.
Source101: aws-signer-notation-root.crt
Source102: aws-us-gov-signer-notation-root.crt

BuildRequires: git
BuildRequires: %{_cross_os}glibc-devel
Requires: %{name}(binaries)
Requires: %{_cross_os}notation
Requires: %{_cross_os}ecr-credential-helper

%description
%{summary}.

%package bin
Summary: AWS Signer plugin for Notation binaries
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(no-fips) and %{name})
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-fips-bin)

%description bin
%{summary}.

%package fips-bin
Summary: AWS Signer plugin for Notation binaries, FIPS edition
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-bin)

%description fips-bin
%{summary}.

%prep
%setup -n %{gorepo}-%{gover} -q
%setup -T -D -n %{gorepo}-%{gover} -b 1 -q

%build
%set_cross_go_flags

go build -ldflags "${GOLDFLAGS}" -o notation-%{plugin_name} ./cmd
gofips build -ldflags "${GOLDFLAGS}" -o fips/notation-%{plugin_name} ./cmd

%install
install -d %{buildroot}{%{_cross_libexecdir},%{_cross_fips_libexecdir},%{_cross_templatedir}}

# Place the binaries where notation expects them.
install -d %{buildroot}%{_cross_libexecdir}/notation-plugins
install -d %{buildroot}%{_cross_libexecdir}/notation-plugins/plugins
install -d %{buildroot}%{_cross_libexecdir}/notation-plugins/plugins/%{plugin_name}

install -d %{buildroot}%{_cross_fips_libexecdir}/notation-plugins
install -d %{buildroot}%{_cross_fips_libexecdir}/notation-plugins/plugins
install -d %{buildroot}%{_cross_fips_libexecdir}/notation-plugins/plugins/%{plugin_name}

install -p -m 0755 notation-%{plugin_name} %{buildroot}%{_cross_libexecdir}/notation-plugins/plugins/%{plugin_name}/notation-%{plugin_name}
install -p -m 0755 fips/notation-%{plugin_name} %{buildroot}%{_cross_fips_libexecdir}/notation-plugins/plugins/%{plugin_name}/notation-%{plugin_name}

# Add the notation config truststore directories.
install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/%{signing_authority_path}/aws-signer-ts/
install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/%{signing_authority_path}/aws-us-gov-signer-ts/
install -p -m 0644 %{S:101} %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/%{signing_authority_path}/aws-signer-ts/
install -p -m 0644 %{S:102} %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/%{signing_authority_path}/aws-us-gov-signer-ts/

%cross_scan_attribution go-vendor vendor

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%dir %{_cross_factorydir}%{_cross_sysconfdir}/%{signing_authority_path}/aws-signer-ts/
%dir %{_cross_factorydir}%{_cross_sysconfdir}/%{signing_authority_path}/aws-us-gov-signer-ts/
%{_cross_factorydir}%{_cross_sysconfdir}/%{signing_authority_path}/aws-signer-ts/aws-signer-notation-root.crt
%{_cross_factorydir}%{_cross_sysconfdir}/%{signing_authority_path}/aws-us-gov-signer-ts/aws-us-gov-signer-notation-root.crt

%files bin
%{_cross_libexecdir}/notation-plugins/plugins/%{plugin_name}/notation-%{plugin_name}

%files fips-bin
%{_cross_fips_libexecdir}/notation-plugins/plugins/%{plugin_name}/notation-%{plugin_name}

%changelog
