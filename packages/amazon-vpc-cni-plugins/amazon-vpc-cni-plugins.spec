%global vpccni_goproject github.com/aws
%global vpccni_gorepo amazon-vpc-cni-plugins
%global vpccni_goimport %{vpccni_goproject}/%{vpccni_gorepo}
%global vpccni_gitrev be5214353252f8315a1341f4df9ffbd8cf69000c
%global vpccni_gover 1.3

Name: %{_cross_os}amazon-vpc-cni-plugins
Version: %{vpccni_gover}
Release: 1%{?dist}
Epoch: 1
Summary: VPC CNI plugins for Amazon ECS and EKS
License: Apache-2.0
URL: https://%{vpccni_goimport}
Source0: https://%{vpccni_goimport}/archive/%{vpccni_gitrev}/%{vpccni_gorepo}.tar.gz

BuildRequires: %{_cross_os}glibc-devel

Requires: %{name}(binaries)
Requires: (%{name}(ecs-agent-extras) if %{_cross_os}variant-family(aws-ecs))

%description
%{summary}.

%package bin
Summary: VPC networking plugins binaries
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(no-fips) and %{name})
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-fips-bin)

%description bin
%{summary}.

%package fips-bin
Summary: VPC networking plugins binaries, FIPS edition
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-bin)

%description fips-bin
%{summary}.

%package ecs-agent-extras
Summary: Extra files necessary for the ECS agent
Provides: %{name}(ecs-agent-extras)
Requires: (%{_cross_os}image-feature(no-fips) and %{name}(binaries))
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-ecs-agent-fips-extras)

%description ecs-agent-extras
%{summary}.

%package ecs-agent-fips-extras
Summary: Extra files necessary for the ECS agent, FIPS edition
Provides: %{name}(ecs-agent-extras)
Requires: (%{_cross_os}image-feature(fips) and %{name}(binaries))
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-ecs-agent-extras)

%description ecs-agent-fips-extras
%{summary}.

%prep
%autosetup -n %{vpccni_gorepo}-%{vpccni_gitrev}

# Symlink amazon-vpc-cni-plugins-%{vpccni_gitrev} to the GOPATH location
%cross_go_setup %{vpccni_gorepo}-%{vpccni_gitrev} %{vpccni_goproject} %{vpccni_goimport}

%build
# cross_go_configure cd's to the correct GOPATH location
%cross_go_configure %{vpccni_goimport}
LD_VPC_CNI_VERSION="-X github.com/aws/amazon-vpc-cni-plugins/version.Version=%{vpccni_gover}"
VPC_CNI_HASH="%{vpccni_gitrev}"
LD_VPC_CNI_SHORT_HASH="-X github.com/aws/amazon-vpc-cni-plugins/version.GitShortHash=${VPC_CNI_HASH::8}"

declare -a VPC_CNI_BUILD_ARGS
VPC_CNI_BUILD_ARGS=(
  -ldflags "${GOLDFLAGS} ${LD_VPC_CNI_VERSION} ${LD_VPC_CNI_SHORT_HASH} ${LD_VPC_CNI_PORCELAIN}"
)

for p in \
  aws-appmesh \
  ecs-serviceconnect \
  vpc-branch-eni \
  vpc-bridge \
  vpc-eni \
  vpc-tunnel \
; do
  go build "${VPC_CNI_BUILD_ARGS[@]}" -mod=vendor -o ${p} ./plugins/${p}
  gofips build "${VPC_CNI_BUILD_ARGS[@]}" -mod=vendor -o fips/${p} ./plugins/${p}
done

%install
install -d %{buildroot}%{_cross_libexecdir}
install -D -p -m 0755 aws-appmesh %{buildroot}%{_cross_libexecdir}/cni/vpc/aws-appmesh
install -D -p -m 0755 ecs-serviceconnect %{buildroot}%{_cross_libexecdir}/cni/vpc/ecs-serviceconnect
install -D -p -m 0755 vpc-branch-eni %{buildroot}%{_cross_libexecdir}/cni/vpc/vpc-branch-eni
install -D -p -m 0755 vpc-bridge %{buildroot}%{_cross_libexecdir}/cni/vpc/vpc-bridge
install -D -p -m 0755 vpc-eni %{buildroot}%{_cross_libexecdir}/cni/vpc/vpc-eni
install -D -p -m 0755 vpc-tunnel %{buildroot}%{_cross_libexecdir}/cni/vpc/vpc-tunnel

install -d %{buildroot}%{_cross_fips_libexecdir}
install -D -p -m 0755 fips/aws-appmesh %{buildroot}%{_cross_fips_libexecdir}/cni/vpc/aws-appmesh
install -D -p -m 0755 fips/ecs-serviceconnect %{buildroot}%{_cross_fips_libexecdir}/cni/vpc/ecs-serviceconnect
install -D -p -m 0755 fips/vpc-branch-eni %{buildroot}%{_cross_fips_libexecdir}/cni/vpc/vpc-branch-eni
install -D -p -m 0755 fips/vpc-bridge %{buildroot}%{_cross_fips_libexecdir}/cni/vpc/vpc-bridge
install -D -p -m 0755 fips/vpc-eni %{buildroot}%{_cross_fips_libexecdir}/cni/vpc/vpc-eni
install -D -p -m 0755 fips/vpc-tunnel %{buildroot}%{_cross_fips_libexecdir}/cni/vpc/vpc-tunnel

# Create symlinks to VPC CNI plugin binaries for amazon-ecs-agent
install -d %{buildroot}{%{_cross_libexecdir},%{_cross_fips_libexecdir}}/amazon-ecs-agent
for p in \
  aws-appmesh \
  ecs-serviceconnect \
  vpc-branch-eni \
  vpc-bridge \
  vpc-eni \
  vpc-tunnel \
; do
  ln -rs %{buildroot}%{_cross_libexecdir}/cni/vpc/${p} %{buildroot}%{_cross_libexecdir}/amazon-ecs-agent/${p}
  ln -rs %{buildroot}%{_cross_fips_libexecdir}/cni/vpc/${p} %{buildroot}%{_cross_fips_libexecdir}/amazon-ecs-agent/${p}
done

%cross_scan_attribution go-vendor vendor

%files
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%license LICENSE

%files bin
%{_cross_libexecdir}/cni/vpc/aws-appmesh
%{_cross_libexecdir}/cni/vpc/ecs-serviceconnect
%{_cross_libexecdir}/cni/vpc/vpc-branch-eni
%{_cross_libexecdir}/cni/vpc/vpc-bridge
%{_cross_libexecdir}/cni/vpc/vpc-eni
%{_cross_libexecdir}/cni/vpc/vpc-tunnel

%files fips-bin
%{_cross_fips_libexecdir}/cni/vpc/aws-appmesh
%{_cross_fips_libexecdir}/cni/vpc/ecs-serviceconnect
%{_cross_fips_libexecdir}/cni/vpc/vpc-branch-eni
%{_cross_fips_libexecdir}/cni/vpc/vpc-bridge
%{_cross_fips_libexecdir}/cni/vpc/vpc-eni
%{_cross_fips_libexecdir}/cni/vpc/vpc-tunnel

%files ecs-agent-extras
%{_cross_libexecdir}/amazon-ecs-agent/aws-appmesh
%{_cross_libexecdir}/amazon-ecs-agent/ecs-serviceconnect
%{_cross_libexecdir}/amazon-ecs-agent/vpc-branch-eni
%{_cross_libexecdir}/amazon-ecs-agent/vpc-bridge
%{_cross_libexecdir}/amazon-ecs-agent/vpc-eni
%{_cross_libexecdir}/amazon-ecs-agent/vpc-tunnel

%files ecs-agent-fips-extras
%{_cross_fips_libexecdir}/amazon-ecs-agent/aws-appmesh
%{_cross_fips_libexecdir}/amazon-ecs-agent/ecs-serviceconnect
%{_cross_fips_libexecdir}/amazon-ecs-agent/vpc-branch-eni
%{_cross_fips_libexecdir}/amazon-ecs-agent/vpc-bridge
%{_cross_fips_libexecdir}/amazon-ecs-agent/vpc-eni
%{_cross_fips_libexecdir}/amazon-ecs-agent/vpc-tunnel

%changelog
