%global ecscni_goproject github.com/aws
%global ecscni_gorepo amazon-ecs-cni-plugins
%global ecscni_goimport %{ecscni_goproject}/%{ecscni_gorepo}
%global ecscni_gitrev 319a734d9edfeeec9e7b64acf0500c45508aa8d3

Name: %{_cross_os}amazon-ecs-cni-plugins
# https://github.com/aws/amazon-ecs-cni-plugins/blob/319a734d9edfeeec9e7b64acf0500c45508aa8d3/VERSION#L1
Version: 2026.02.0
Release: 1%{?dist}
Epoch: 1
Summary: Networking plugins for ECS task networking
License: Apache-2.0
URL: https://%{ecscni_goimport}
Source0: https://%{ecscni_goimport}/archive/%{ecscni_gitrev}/%{ecscni_gorepo}.tar.gz

# Bottlerocket-specific - filesystem location for ECS CNI plugins
Patch0001: 0001-bottlerocket-default-filesystem-locations.patch

BuildRequires: %{_cross_os}glibc-devel

Requires: (%{name}-ecs-agent-extras if %{_cross_os}variant-family(aws-ecs))

%description
%{summary}.

%package ecs-agent-extras
Summary: Extra files necessary for the ECS agent
Requires: %{name}

%description ecs-agent-extras
%{summary}.

%prep
%autosetup -n %{ecscni_gorepo}-%{ecscni_gitrev} -p1

# Symlink amazon-ecs-cni-plugins-%{ecscni_gitrev} to the GOPATH location
%cross_go_setup %{ecscni_gorepo}-%{ecscni_gitrev} %{ecscni_goproject} %{ecscni_goimport}

%build
# cross_go_configure cd's to the correct GOPATH location
%cross_go_configure %{ecscni_goimport}
LD_ECS_CNI_VERSION="-X github.com/aws/amazon-ecs-cni-plugins/pkg/version.Version=$(cat VERSION)"
ECS_CNI_HASH="%{ecscni_gitrev}"
LD_ECS_CNI_SHORT_HASH="-X github.com/aws/amazon-ecs-cni-plugins/pkg/version.GitShortHash=${ECS_CNI_HASH::8}"
LD_ECS_CNI_PORCELAIN="-X github.com/aws/amazon-ecs-cni-plugins/pkg/version.GitPorcelain=0"

declare -a ECS_CNI_BUILD_ARGS
ECS_CNI_BUILD_ARGS=(
  -ldflags "${GOLDFLAGS} ${LD_ECS_CNI_VERSION} ${LD_ECS_CNI_SHORT_HASH} ${LD_ECS_CNI_PORCELAIN}"
)

go build "${ECS_CNI_BUILD_ARGS[@]}" -o ecs-eni ./plugins/eni

go build "${ECS_CNI_BUILD_ARGS[@]}" -o ecs-ipam ./plugins/ipam

go build "${ECS_CNI_BUILD_ARGS[@]}" -o ecs-bridge ./plugins/ecs-bridge

%install
install -d %{buildroot}%{_cross_libexecdir}
install -D -p -m 0755 ecs-bridge %{buildroot}%{_cross_libexecdir}/cni/ecs/ecs-bridge
install -D -p -m 0755 ecs-eni %{buildroot}%{_cross_libexecdir}/cni/ecs/ecs-eni
install -D -p -m 0755 ecs-ipam %{buildroot}%{_cross_libexecdir}/cni/ecs/ecs-ipam

# Create symlinks to ECS CNI plugin binaries for amazon-ecs-agent
install -d %{buildroot}%{_cross_libexecdir}/amazon-ecs-agent
for p in \
  ecs-bridge \
  ecs-eni \
  ecs-ipam \
; do
  ln -rs %{buildroot}%{_cross_libexecdir}/cni/ecs/${p} %{buildroot}%{_cross_libexecdir}/amazon-ecs-agent/${p}
done

%cross_scan_attribution go-vendor vendor

%files
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%license LICENSE
%{_cross_libexecdir}/cni/ecs/ecs-bridge
%{_cross_libexecdir}/cni/ecs/ecs-eni
%{_cross_libexecdir}/cni/ecs/ecs-ipam

%files ecs-agent-extras
%{_cross_libexecdir}/amazon-ecs-agent/ecs-bridge
%{_cross_libexecdir}/amazon-ecs-agent/ecs-eni
%{_cross_libexecdir}/amazon-ecs-agent/ecs-ipam

%changelog
