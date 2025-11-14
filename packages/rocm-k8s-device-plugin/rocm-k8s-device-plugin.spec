%global goproject github.com/ROCm
%global gorepo k8s-device-plugin
%global goimport %{goproject}/%{gorepo}

%global gover 1.31.0.8
%global rpmver %{gover}

Name: %{_cross_os}rocm-k8s-device-plugin
Version: %{rpmver}
Release: 1%{?dist}
Summary: Kubernetes device plugin for AMD GPUs
License: Apache-2.0
URL: https://github.com/ROCm/k8s-device-plugin
Source0: https://github.com/ROCm/k8s-device-plugin/archive/v%{gover}.tar.gz
Source1: rocm-k8s-device-plugin.service

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libdrm-devel
BuildRequires: %{_cross_os}hwloc-devel
Requires: %{name}(binaries)
Requires: %{_cross_os}libdrm
Requires: %{_cross_os}hwloc

%description
%{summary}.

%package bin
Summary: Kubernetes device plugin for AMD GPUs binaries
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(no-fips) and %{name})
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-fips-bin)

%description bin
%{summary}.

%package fips-bin
Summary: Kubernetes device plugin for AMD GPUs binaries, FIPS edition
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-bin)

%description fips-bin
%{summary}.

%prep
%autosetup -n %{gorepo}-%{gover} -p1
%cross_go_setup %{gorepo}-%{gover} %{goproject} %{goimport}

%build
%cross_go_configure %{goimport}

go build -ldflags="${GOLDFLAGS}" -o  rocm-device-plugin ./cmd/k8s-device-plugin/
gofips build -ldflags="${GOLDFLAGS}" -o fips/rocm-device-plugin ./cmd/k8s-device-plugin/

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 rocm-device-plugin %{buildroot}%{_cross_bindir}

install -d %{buildroot}%{_cross_fips_bindir}
install -p -m 0755 fips/rocm-device-plugin %{buildroot}%{_cross_fips_bindir}

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:1} %{buildroot}%{_cross_unitdir}

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_unitdir}/rocm-k8s-device-plugin.service

%files bin
%{_cross_bindir}/rocm-device-plugin

%files fips-bin
%{_cross_fips_bindir}/rocm-device-plugin
