%global goproject github.com/awslabs
%global gorepo amazon-ecr-credential-helper
%global goimport %{goproject}/%{gorepo}

%global gover 0.10.1
%global rpmver %{gover}

%global _dwz_low_mem_die_limit 0

Name: %{_cross_os}ecr-credential-helper
Version: %{rpmver}
Release: 1%{?dist}
Summary: Amazon ECR credential helper
License: Apache-2.0
URL: https://%{goimport}
Source0: https://%{goimport}/archive/v%{gover}/%{gorepo}-%{gover}.tar.gz
Source1: bundled-%{gorepo}-%{gover}.tar.gz

Source10: ecr-credential-helper-tmpfiles.conf
Source11: root-.docker.mount
Source12: root-.ecr.mount
Source13: docker-root-config.json

BuildRequires: %{_cross_os}glibc-devel
Requires: %{name}(binaries)

%description
%{summary}.

%package bin
Summary: Amazon ECR credential helper binaries
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(no-fips) and %{name})
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-fips-bin)

%description bin
%{summary}.

%package fips-bin
Summary: Amazon ECR credential helper binaries, FIPS edition
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-bin)

%description fips-bin
%{summary}.

%prep
%autosetup -n %{gorepo}-%{gover} -p1
%cross_go_setup %{gorepo}-%{gover} %{goproject} %{goimport}

%build
# cross_go_configure cd's to the correct GOPATH location
%cross_go_configure %{goimport}

go build -ldflags="${GOLDFLAGS}" -o=docker-credential-ecr-login ./ecr-login/cli/docker-credential-ecr-login
gofips build -ldflags="${GOLDFLAGS}" -o=fips/docker-credential-ecr-login ./ecr-login/cli/docker-credential-ecr-login

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 docker-credential-ecr-login %{buildroot}%{_cross_bindir}

install -d %{buildroot}%{_cross_fips_bindir}
install -p -m 0755 fips/docker-credential-ecr-login %{buildroot}%{_cross_fips_bindir}

install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m0644 %{S:10} %{buildroot}%{_cross_tmpfilesdir}/ecr-credential-helper.conf

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:11} %{S:12} %{buildroot}%{_cross_unitdir}

install -d %{buildroot}%{_cross_factorydir}/root/.docker
install -p -m0600 %{S:13} %{buildroot}%{_cross_factorydir}/root/.docker/config.json

%cross_scan_attribution go-vendor ./ecr-login/vendor

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%{_cross_factorydir}/root/.docker/config.json
%{_cross_tmpfilesdir}/ecr-credential-helper.conf
%{_cross_unitdir}/root-.docker.mount
%{_cross_unitdir}/root-.ecr.mount

%files bin
%{_cross_bindir}/docker-credential-ecr-login

%files fips-bin
%{_cross_fips_bindir}/docker-credential-ecr-login
