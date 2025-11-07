%global goproject github.com/containerd
%global gorepo containerd
%global goimport %{goproject}/%{gorepo}

%global gover 2.1.5
%global rpmver %{gover}
%global gitrev fcd43222d6b07379a4be9786bda52438f0dd16a1

%global package_priority_epoch 0
%global _dwz_low_mem_die_limit 0

Name: %{_cross_os}%{gorepo}-2.1
Version: %{rpmver}
Release: 1%{?dist}
Summary: An industry-standard container runtime
License: Apache-2.0
URL: https://%{goimport}
Source0: https://%{goimport}/archive/v%{gover}/%{gorepo}-%{gover}.tar.gz
Source1: containerd.service
Source2: containerd-config-toml_k8s_containerd_sock
Source3: containerd-config-toml_basic
Source4: containerd-config-toml_k8s_nvidia_containerd_sock
Source5: containerd-tmpfiles.conf
Source6: containerd-cri-base-json
Source7: snapshotter-toml

# Mount for writing containerd configuration
Source100: etc-containerd.mount

# Create container storage mount point.
Source110: prepare-var-lib-containerd.service

# Drop-ins to disable igzip or pigz if the other implementation is preferred.
Source200: containerd-disable-igzip.conf
Source201: containerd-disable-pigz.conf

Source1000: clarify.toml

# Patch to support moving from containerd-1.7 to 2.x
Patch1001: 1001-Revert-Don-t-allow-io_uring-related-syscalls-in-the-.patch

BuildRequires: git
BuildRequires: %{_cross_os}glibc-devel
Requires: %{_cross_os}runc
Requires: %{name}(optimized-gunzip)
Requires: %{name}(binaries)

Provides: %{_cross_os}%{gorepo} = %{package_priority_epoch}:
Conflicts: %{_cross_os}%{gorepo}

%description
%{summary}.

%package bin
Summary: An industry-standard container runtime's binaries
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(no-fips) and %{name})
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-fips-bin)

%description bin
%{summary}.

%package fips-bin
Summary: An industry-standard container runtime's binaries, FIPS edition
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-bin)

%description fips-bin
%{summary}.

%package pigz
Summary: Prefer pigz for gzip decompression
Requires: %{_cross_os}pigz
Requires: %{name}
Provides: %{_cross_os}%{gorepo}-pigz = %{package_priority_epoch}:
Conflicts: %{name}-igzip
Provides: %{name}(optimized-gunzip) = 1:

%description pigz
%{summary}.

%package igzip
Summary: Prefer igzip for gzip decompression
Requires: %{_cross_os}igzip
Requires: %{name}
Provides: %{_cross_os}%{gorepo}-igzip = %{package_priority_epoch}:
Conflicts: %{name}-pigz
%if "%{_cross_arch}" == "x86_64"
Provides: %{name}(optimized-gunzip) = 2:
%else
Provides: %{name}(optimized-gunzip) = 0:
%endif

%description igzip
%{summary}.

%prep
%autosetup -Sgit -n %{gorepo}-%{gover} -p1

%build
%set_cross_go_flags

export BUILDTAGS="no_btrfs selinux"
export LD_VERSION="-X github.com/containerd/containerd/v2/version.Version=%{gover}+bottlerocket"
export LD_REVISION="-X github.com/containerd/containerd/v2/version.Revision=%{gitrev}"

declare -a BUILD_ARGS
BUILD_ARGS=(
  -tags="${BUILDTAGS}"
  -ldflags="${GOLDFLAGS} ${LD_VERSION} ${LD_REVISION}"
)

for bin in \
  containerd \
  containerd-shim-runc-v2 \
  ctr ;
do
  go build "${BUILD_ARGS[@]}" -o ${bin} ./cmd/${bin}
  gofips build "${BUILD_ARGS[@]}" -o fips/${bin} ./cmd/${bin}
done

%install
install -d %{buildroot}{%{_cross_bindir},%{_cross_fips_bindir}}
for bin in \
  containerd \
  containerd-shim-runc-v2 \
  ctr ;
do
  install -p -m 0755 ${bin} %{buildroot}%{_cross_bindir}
  install -p -m 0755 fips/${bin} %{buildroot}%{_cross_fips_bindir}
done

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:1} %{S:100} %{S:110} %{buildroot}%{_cross_unitdir}

install -d %{buildroot}%{_cross_templatedir}
install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/containerd
install -p -m 0644 %{S:2} %{S:3} %{S:4} %{S:6} %{S:7} %{buildroot}%{_cross_templatedir}

install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m 0644 %{S:5} %{buildroot}%{_cross_tmpfilesdir}/containerd.conf

install -d %{buildroot}%{_cross_unitdir}/containerd.service.d
install -p -m 0644 %{S:200} %{buildroot}%{_cross_unitdir}/containerd.service.d/005-disable-igzip.conf
install -p -m 0644 %{S:201} %{buildroot}%{_cross_unitdir}/containerd.service.d/005-disable-pigz.conf

%cross_scan_attribution --clarify %{S:1000} go-vendor vendor

%files
%license LICENSE NOTICE
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%{_cross_unitdir}/containerd.service
%{_cross_unitdir}/etc-containerd.mount
%{_cross_unitdir}/prepare-var-lib-containerd.service
%dir %{_cross_factorydir}%{_cross_sysconfdir}/containerd
%{_cross_templatedir}/containerd-config-toml*
%{_cross_templatedir}/containerd-cri-base-json
%{_cross_templatedir}/snapshotter-toml
%{_cross_tmpfilesdir}/containerd.conf

%files bin
%{_cross_bindir}/containerd
%{_cross_bindir}/containerd-shim-runc-v2
%{_cross_bindir}/ctr

%files fips-bin
%{_cross_fips_bindir}/containerd
%{_cross_fips_bindir}/containerd-shim-runc-v2
%{_cross_fips_bindir}/ctr

%files pigz
%{_cross_unitdir}/containerd.service.d/005-disable-igzip.conf

%files igzip
%{_cross_unitdir}/containerd.service.d/005-disable-pigz.conf

%changelog
