%global _cross_first_party 1
%undefine _debugsource_packages

Name: %{_cross_os}ecr-credential-helper-shim
Version: 0.1.0
Release: 1%{?dist}
Summary: FIPS shim for ECR credential helper
License: Apache-2.0 OR MIT
URL: https://github.com/bottlerocket-os/bottlerocket
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%prep
%setup -T -c
%cargo_prep

%build
%cargo_build --manifest-path %{_builddir}/sources/Cargo.toml \
    -p ecr-credential-helper-shim \
    --target-dir=${HOME}/.cache/ecr-credential-helper-shim

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 ${HOME}/.cache/ecr-credential-helper-shim/%{__cargo_target}/release/ecr-credential-helper-shim %{buildroot}%{_cross_bindir}/docker-credential-ecr-login

%files
%{_cross_bindir}/docker-credential-ecr-login
