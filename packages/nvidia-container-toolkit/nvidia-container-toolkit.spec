%global goproject github.com/NVIDIA
%global gorepo nvidia-container-toolkit
%global goimport %{goproject}/%{gorepo}

%global gover 1.18.0
%global rpmver %{gover}

Name: %{_cross_os}nvidia-container-toolkit
Version: %{rpmver}
Release: 1%{?dist}
Epoch: 1
Summary: Tool to build and run GPU accelerated containers
License: Apache-2.0
URL: https://%{goimport}

Source0: https://%{goimport}/archive/v%{gover}/nvidia-container-toolkit-%{gover}.tar.gz
# non-templated version of the config files for k8s are provided for downstream
# builders that don't use the NVIDIA Container Runtime settings
Source1: nvidia-container-toolkit-config-k8s.toml
Source2: nvidia-container-toolkit-config-ecs.toml
Source3: nvidia-gpu-devices.rules
Source4: nvidia-container-toolkit-tmpfiles-ecs.conf
Source5: nvidia-container-toolkit-tmpfiles-k8s.conf
Source6: nvidia-container-toolkit-config-k8s
Source7: generate-cdi-specs.service

BuildRequires: %{_cross_os}glibc-devel
Requires: %{_cross_os}libnvidia-container
Requires: (%{name}-ecs if %{_cross_os}variant-family(aws-ecs))
Requires: (%{name}-k8s if %{_cross_os}variant-family(aws-k8s))

%description
%{summary}.

%package bin
Summary: NVIDIA container toolkit binaries
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(no-fips) and %{name})
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-fips-bin)

%description bin
%{summary}.

%package fips-bin
Summary: NVIDIA container toolkit binaries, FIPS edition
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-bin)

%description fips-bin
%{summary}.

%package ecs
Summary: Files specific for the ECS variants
Requires: %{name}
Conflicts: %{name}-k8s

%description ecs
%{summary}.

%package k8s
Summary: Files specific for the Kubernetes variants
Requires: %{name}
Conflicts: %{name}-ecs

%description k8s
%{summary}.

%prep
%autosetup -n %{gorepo}-%{gover} -p1
%cross_go_setup %{gorepo}-%{gover} %{goproject} %{goimport}

%build
%cross_go_configure %{goimport}

# We don't set `-Wl,-z,now`, because the binary uses lazy loading
# to load the NVIDIA libraries in the host
export CGO_LDFLAGS="-Wl,-z,relro -Wl,--export-dynamic"
export GOLDFLAGS="-compressdwarf=false -linkmode=external -extldflags '${CGO_LDFLAGS}'"

for bin in \
  nvidia-cdi-hook \
  nvidia-container-runtime-hook \
  nvidia-container-runtime \
  nvidia-container-runtime.cdi \
  nvidia-container-runtime.legacy \
  nvidia-ctk ;
do
  go build -ldflags="${GOLDFLAGS}" -o ${bin} ./cmd/${bin}
  gofips build -ldflags="${GOLDFLAGS}" -o fips/${bin} ./cmd/${bin}
done

%install
install -d %{buildroot}%{_cross_bindir}
install -d %{buildroot}%{_cross_fips_bindir}
install -d %{buildroot}%{_cross_tmpfilesdir}
install -d %{buildroot}%{_cross_templatedir}
install -d %{buildroot}%{_cross_udevrulesdir}
install -d %{buildroot}%{_cross_unitdir}
install -d %{buildroot}%{_cross_datadir}/nvidia-container-toolkit
install -d %{buildroot}%{_cross_factorydir}/nvidia-container-runtime
install -d %{buildroot}%{_cross_templatedir}/nvidia-container-runtime

for bin in \
  nvidia-cdi-hook \
  nvidia-container-runtime-hook \
  nvidia-container-runtime \
  nvidia-container-runtime.cdi \
  nvidia-container-runtime.legacy \
  nvidia-ctk ;
do
  install -p -m 0755 ${bin} %{buildroot}%{_cross_bindir}
  install -p -m 0755 fips/${bin} %{buildroot}%{_cross_fips_bindir}
done

install -m 0644 %{S:1} %{buildroot}%{_cross_factorydir}/nvidia-container-runtime/
install -m 0644 %{S:2} %{buildroot}%{_cross_factorydir}/nvidia-container-runtime/
install -p -m 0644 %{S:3} %{buildroot}%{_cross_udevrulesdir}/90-nvidia-gpu-devices.rules
install -m 0644 %{S:4} %{buildroot}%{_cross_tmpfilesdir}/nvidia-container-toolkit-ecs.conf
install -m 0644 %{S:5} %{buildroot}%{_cross_tmpfilesdir}/nvidia-container-toolkit-k8s.conf
install -m 0644 %{S:6} %{buildroot}%{_cross_templatedir}/nvidia-container-runtime/
install -m 0644 %{S:7} %{buildroot}%{_cross_unitdir}/

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_udevrulesdir}/90-nvidia-gpu-devices.rules
%{_cross_unitdir}/generate-cdi-specs.service

%files bin
%{_cross_bindir}/nvidia-container-runtime-hook
%{_cross_bindir}/nvidia-cdi-hook
%{_cross_bindir}/nvidia-container-runtime
%{_cross_bindir}/nvidia-container-runtime.cdi
%{_cross_bindir}/nvidia-container-runtime.legacy
%{_cross_bindir}/nvidia-ctk

%files fips-bin
%{_cross_fips_bindir}/nvidia-container-runtime-hook
%{_cross_fips_bindir}/nvidia-cdi-hook
%{_cross_fips_bindir}/nvidia-container-runtime
%{_cross_fips_bindir}/nvidia-container-runtime.cdi
%{_cross_fips_bindir}/nvidia-container-runtime.legacy
%{_cross_fips_bindir}/nvidia-ctk

%files ecs
%{_cross_factorydir}/nvidia-container-runtime/nvidia-container-toolkit-config-ecs.toml
%{_cross_tmpfilesdir}/nvidia-container-toolkit-ecs.conf

%files k8s
%{_cross_factorydir}/nvidia-container-runtime/nvidia-container-toolkit-config-k8s.toml
%{_cross_templatedir}/nvidia-container-runtime/nvidia-container-toolkit-config-k8s
%{_cross_tmpfilesdir}/nvidia-container-toolkit-k8s.conf
