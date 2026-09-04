%global goproject github.com/NVIDIA
%global gorepo k8s-device-plugin
%global goimport %{goproject}/%{gorepo}

%global gover 0.19.3
%global rpmver %{gover}

Name: %{_cross_os}nvidia-k8s-device-plugin
Version: %{rpmver}
Release: 1%{?dist}
Epoch: 1
Summary: Kubernetes device plugin for NVIDIA GPUs
License: Apache-2.0
URL: https://github.com/NVIDIA/k8s-device-plugin
Source0: https://%{goimport}/archive/v%{gover}/v%{gover}.tar.gz#/k8s-device-plugin-%{gover}.tar.gz
Source1: nvidia-k8s-device-plugin.service
Source2: nvidia-k8s-device-plugin-conf.in
Source3: nvidia-k8s-device-plugin-exec-start-conf
Source4: nvidia-k8s-device-plugin-mig-conf
Source5: nvidia-mps-control-daemon.service
Source6: nvidia-mps-control-daemon-exec-start-conf

Patch0001: 0001-Update-MPS-roots-for-immutable-host-OS.patch
Patch1001: 1001-Ensure-that-generated-CDI-specs-do-not-contain-enabl.patch
Patch1002: 1002-Enable-library-path-compatibility-symlinks.patch
Patch1003: 1003-vendor-add-CreateLibSymlinksHook.patch

BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%prep
%autosetup -n %{gorepo}-%{gover} -p1
%cross_go_setup %{gorepo}-%{gover} %{goproject} %{goimport}

%build
%cross_go_configure %{goimport}
export GO_MAJOR="1.26"
# We don't set `-Wl,-z,now`, because the binary uses lazy loading
# to load the NVIDIA libraries in the host
export CGO_LDFLAGS="-Wl,-z,relro -Wl,--export-dynamic"
export GOLDFLAGS="-compressdwarf=false -linkmode=external -extldflags '${CGO_LDFLAGS}'"

go build -ldflags="${GOLDFLAGS}" -o nvidia-device-plugin ./cmd/nvidia-device-plugin/
go build -ldflags="${GOLDFLAGS}" -o mps-control-daemon ./cmd/mps-control-daemon/

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 nvidia-device-plugin %{buildroot}%{_cross_bindir}
install -p -m 0755 mps-control-daemon %{buildroot}%{_cross_bindir}

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:1} %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:5} %{buildroot}%{_cross_unitdir}
install -d %{buildroot}%{_cross_unitdir}/nvidia-k8s-device-plugin.service.d
install -d %{buildroot}%{_cross_unitdir}/nvidia-mps-control-daemon.service.d
sed -e 's|__PREFIX__|/%{_cross_arch}-bottlerocket-linux-gnu/sys-root|g' %{S:2} > nvidia-k8s-device-plugin-conf
install -D -m 0644 nvidia-k8s-device-plugin-conf %{buildroot}%{_cross_templatedir}/nvidia-k8s-device-plugin-conf
install -D -m 0644 %{S:3} %{buildroot}%{_cross_templatedir}/nvidia-k8s-device-plugin-exec-start-conf
install -D -m 0644 %{S:4} %{buildroot}%{_cross_templatedir}/nvidia-k8s-device-plugin-mig-conf
install -D -m 0644 %{S:6} %{buildroot}%{_cross_templatedir}/nvidia-mps-control-daemon-exec-start-conf

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_bindir}/nvidia-device-plugin
%{_cross_bindir}/mps-control-daemon
%{_cross_unitdir}/nvidia-k8s-device-plugin.service
%{_cross_unitdir}/nvidia-mps-control-daemon.service
%dir %{_cross_unitdir}/nvidia-k8s-device-plugin.service.d
%dir %{_cross_unitdir}/nvidia-mps-control-daemon.service.d
%{_cross_templatedir}/nvidia-k8s-device-plugin-conf
%{_cross_templatedir}/nvidia-k8s-device-plugin-exec-start-conf
%{_cross_templatedir}/nvidia-k8s-device-plugin-mig-conf
%{_cross_templatedir}/nvidia-mps-control-daemon-exec-start-conf

