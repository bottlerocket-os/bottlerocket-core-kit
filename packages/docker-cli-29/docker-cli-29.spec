%global goproject github.com/docker
%global gorepo cli
%global goimport %{goproject}/%{gorepo}

%global gover 29.0.4
%global rpmver %{gover}
%global gitrev 3247a5aae3791c8306f5b2e215c314267c31c570

%global source_date_epoch 1492525740

%global _dwz_low_mem_die_limit 0

%global package_priority_epoch 0

Name: %{_cross_os}docker-%{gorepo}-29
Version: %{rpmver}
Release: 1%{?dist}
Summary: Docker CLI
License: Apache-2.0
URL: https://%{goimport}
Source0: https://%{goimport}/archive/v%{gover}/cli-%{gover}.tar.gz
Source1000: clarify.toml

BuildRequires: git
BuildRequires: %{_cross_os}glibc-devel

Provides: %{_cross_os}docker-%{gorepo} = %{package_priority_epoch}:
Conflicts: %{_cross_os}docker-%{gorepo}

%description
%{summary}.

%prep
%autosetup -Sgit -n %{gorepo}-%{gover} -p1
%cross_go_setup %{gorepo}-%{gover} %{goproject} %{goimport}

%build
%cross_go_configure %{goimport}
LD_VERSION="-X github.com/docker/cli/cli/version.Version=%{gover}"
LD_GIT_REV="-X github.com/docker/cli/cli/version.GitCommit=%{gitrev}"
LD_PLATFORM="-X \"github.com/docker/cli/cli/version.PlatformName=Docker Engine - Community\""
BUILDTIME=$(date -u -d "@%{source_date_epoch}" --rfc-3339 ns 2> /dev/null | sed -e 's/ /T/')
LD_BUILDTIME="-X github.com/docker/cli/cli/version.BuildTime=${BUILDTIME}"

declare -a BUILD_ARGS
BUILD_ARGS=(
  -ldflags="${GOLDFLAGS} ${LD_VERSION} ${LD_GIT_REV} ${LD_PLATFORM} ${LD_BUILDTIME}"
)

go build "${BUILD_ARGS[@]}" -o docker %{goimport}/cmd/docker

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 docker %{buildroot}%{_cross_bindir}

%cross_scan_attribution --clarify %{S:1000} go-vendor vendor

%files
%license LICENSE NOTICE
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%{_cross_bindir}/docker
%changelog
