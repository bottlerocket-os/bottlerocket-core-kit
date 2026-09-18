# The upstream repository is kubernetes-sigs/dra-driver-nvidia-gpu, but the Go
# module path (and therefore the import path the sources compile against) is
# still github.com/NVIDIA/k8s-dra-driver-gpu.
%global goproject github.com/NVIDIA
%global goimport %{goproject}/k8s-dra-driver-gpu

# Name of the upstream repo, used for the archive directory / source filename.
%global gorepo dra-driver-nvidia-gpu

%global gover 25.8.1
%global rpmver %{gover}

Name: %{_cross_os}nvidia-dra-driver-gpu
Version: %{rpmver}
Release: 1%{?dist}
Summary: NVIDIA DRA driver for Kubernetes GPUs
License: Apache-2.0
URL: https://github.com/kubernetes-sigs/dra-driver-nvidia-gpu
Source0: https://github.com/kubernetes-sigs/%{gorepo}/archive/v%{gover}/%{gorepo}-%{gover}.tar.gz
Source1: nvidia-dra-driver-gpu.service
Source2: nvidia-dra-driver-gpu-exec-start-conf
Source3: nvidia-dra-driver-gpu-enabled-marker

BuildRequires: %{_cross_os}glibc-devel
# The rendered unit invokes /usr/bin/nvidia-cdi-hook, shipped by the toolkit.
Requires: %{_cross_os}nvidia-container-toolkit

%description
%{summary}.

%prep
%autosetup -n %{gorepo}-%{gover} -p1
%cross_go_setup %{gorepo}-%{gover} %{goproject} %{goimport}

%build
%cross_go_configure %{goimport}
export GO_MAJOR="1.26"
# The plugin lazily loads (dlopen) the NVIDIA management libraries from the
# host, so we don't set `-Wl,-z,now`; we export dynamic symbols instead.
export CGO_LDFLAGS="-Wl,-z,relro -Wl,--export-dynamic"
export GOLDFLAGS="-compressdwarf=false -linkmode=external -extldflags '${CGO_LDFLAGS}'"

go build -ldflags="${GOLDFLAGS} -X %{goimport}/internal/info.version=v%{gover}" \
    -o gpu-kubelet-plugin ./cmd/gpu-kubelet-plugin/

%install
# Upstream vendors its Go dependencies, so scan them for attribution.
%cross_scan_attribution go-vendor vendor

install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 gpu-kubelet-plugin %{buildroot}%{_cross_bindir}

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:1} %{buildroot}%{_cross_unitdir}
install -d %{buildroot}%{_cross_unitdir}/nvidia-dra-driver-gpu.service.d

install -D -m 0644 %{S:2} %{buildroot}%{_cross_templatedir}/nvidia-dra-driver-gpu-exec-start-conf
install -D -m 0644 %{S:3} %{buildroot}%{_cross_templatedir}/nvidia-dra-driver-gpu-enabled-marker

%files
%license LICENSE NOTICE
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%{_cross_bindir}/gpu-kubelet-plugin
%{_cross_unitdir}/nvidia-dra-driver-gpu.service
%dir %{_cross_unitdir}/nvidia-dra-driver-gpu.service.d
%{_cross_templatedir}/nvidia-dra-driver-gpu-exec-start-conf
%{_cross_templatedir}/nvidia-dra-driver-gpu-enabled-marker
