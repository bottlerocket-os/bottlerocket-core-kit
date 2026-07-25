%global goproject github.com/aws
%global gorepo rolesanywhere-credential-helper
%global goimport %{goproject}/%{gorepo}

%global gover 1.8.2
%global rpmver %{gover}

%global _dwz_low_mem_die_limit 0

Name: %{_cross_os}aws-signing-helper
Version: %{rpmver}
Release: 1%{?dist}
Epoch: 1
Summary: AWS signing helper for IAM Roles Anywhere support
License: Apache-2.0
URL: https://github.com/aws/rolesanywhere-credential-helper

Source: rolesanywhere-credential-helper-v%{gover}.tar.gz
Source1: bundled-rolesanywhere-credential-helper-v%{gover}.tar.gz
Source2: brush-aws-signing-helper.toml
Source1000: clarify.toml

BuildRequires: %{_cross_os}glibc-devel

# The AWS SDK for GO needs a program to handle `sh -c` invocations in order to
# run credential processes.
Requires: %{_cross_os}package-file(/bin/sh)

%description
%{summary}.

%prep
%setup -n %{gorepo}-%{gover} -q
%setup -T -D -n %{gorepo}-%{gover} -b 1 -q

%build
%set_cross_go_flags
export GO_MAJOR="1.26"

go build -ldflags "-X 'main.Version=${gover}' ${GOLDFLAGS}" -o aws-signing-helper main.go

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 aws-signing-helper %{buildroot}%{_cross_bindir}/aws_signing_helper
ln -sf aws_signing_helper %{buildroot}%{_cross_bindir}/aws-signing-helper

install -d %{buildroot}%{_cross_libexecdir}/brush/allowed-programs
ln -srf \
  %{buildroot}%{_cross_bindir}/aws_signing_helper \
  %{buildroot}%{_cross_libexecdir}/brush/allowed-programs/aws_signing_helper
ln -sf \
  aws_signing_helper \
  %{buildroot}%{_cross_libexecdir}/brush/allowed-programs/aws-signing-helper

install -d %{buildroot}%{_cross_datadir}/brush
install -p -m 0755 %{S:2} %{buildroot}%{_cross_datadir}/brush/aws_signing_helper.toml
ln -sf aws_signing_helper.toml %{buildroot}%{_cross_datadir}/brush/aws-signing-helper.toml

%cross_scan_attribution --clarify %{S:1000} go-vendor vendor

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%{_cross_bindir}/aws_signing_helper
%{_cross_bindir}/aws-signing-helper
%{_cross_datadir}/brush/aws_signing_helper.toml
%{_cross_datadir}/brush/aws-signing-helper.toml
%{_cross_libexecdir}/brush/allowed-programs/aws_signing_helper
%{_cross_libexecdir}/brush/allowed-programs/aws-signing-helper

