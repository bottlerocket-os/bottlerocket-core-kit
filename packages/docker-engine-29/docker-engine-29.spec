%global project moby
%global repo github.com/moby/%{project}
%global goorg github.com/moby
%global goimport %{goorg}/moby

%global gover 29.0.4
%global rpmver %{gover}
%global gitrev 4612690e23f7c4200af175e12cae206b2ee00c7b

%global source_date_epoch 1363394400

%global _dwz_low_mem_die_limit 0

%global package_priority_epoch 0

Name: %{_cross_os}docker-engine-29
Version: %{rpmver}
Release: 1%{?dist}
Summary: Docker engine
License: Apache-2.0
URL: https://%{repo}
Source0: https://%{repo}/archive/docker-v%{gover}/%{project}-docker-v%{gover}.tar.gz
Source1: docker.service
Source2: docker.socket
Source3: docker-sysusers.conf
Source4: daemon-json
Source5: daemon-nvidia-json
Source6: docker-engine-tmpfiles.conf
Source7: ephemeral-storage.conf

# Create container storage mount point.
Source100: prepare-var-lib-docker.service

Source1000: clarify.toml

Patch0001: 0001-Change-default-capabilities-using-daemon-config.patch
Patch0002: 0002-oci-inject-kmod-in-all-containers.patch
Patch0003: 0003-Switch-containerd-image-backend-s-image-pull-to-tran.patch
Patch0004: 0004-Set-label-for-containerd-overlayfs-mounts.patch

BuildRequires: git
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libseccomp-devel
BuildRequires: %{_cross_os}systemd-devel
BuildRequires: %{_cross_os}nftables-devel
Requires: %{_cross_os}containerd
Requires: %{_cross_os}libseccomp
Requires: %{_cross_os}systemd
Requires: %{_cross_os}procps
Requires: %{_cross_os}nftables

Provides: %{_cross_os}docker-engine = %{package_priority_epoch}:
Conflicts: %{_cross_os}docker-engine

%description
%{summary}.

%prep
%autosetup -Sgit -n %{project}-docker-v%{gover} -p1
%cross_go_setup %{project}-docker-v%{gover} %{goorg} %{goimport}

%build
%cross_go_configure %{goimport}
BUILDTAGS="journald selinux seccomp"
BUILDTAGS+=" exclude_graphdriver_btrfs"
BUILDTAGS+=" exclude_graphdriver_devicemapper"
BUILDTAGS+=" exclude_graphdriver_vfs"
BUILDTAGS+=" exclude_graphdriver_zfs"
export BUILDTAGS
export VERSION=%{gover}
export GITCOMMIT=%{gitrev}
export BUILDTIME=$(date -u -d "@%{source_date_epoch}" --rfc-3339 ns 2> /dev/null | sed -e 's/ /T/')
export PLATFORM="Docker Engine - Community"
source ./hack/make/.go-autogen

declare -a BUILD_ARGS
BUILD_ARGS=(
  -ldflags="${GOLDFLAGS} ${LDFLAGS}"
  -tags="${BUILDTAGS}"
)

go build "${BUILD_ARGS[@]}" -o dockerd %{goimport}/v2/cmd/dockerd
go build "${BUILD_ARGS[@]}" -o docker-proxy %{goimport}/v2/cmd/docker-proxy

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 dockerd %{buildroot}%{_cross_bindir}
install -p -m 0755 docker-proxy %{buildroot}%{_cross_bindir}

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:1} %{S:100} %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:2} %{buildroot}%{_cross_unitdir}/docker.socket

install -d %{buildroot}%{_cross_sysusersdir}
install -p -m 0644 %{S:3} %{buildroot}%{_cross_sysusersdir}/docker.conf

install -d %{buildroot}%{_cross_templatedir}
install -p -m 0644 %{S:4} %{buildroot}%{_cross_templatedir}/docker-daemon-json
install -p -m 0644 %{S:5} %{buildroot}%{_cross_templatedir}/docker-daemon-nvidia-json

install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m 0644 %{S:6} %{buildroot}%{_cross_tmpfilesdir}/docker-engine.conf

install -D -p -m 0644 %{S:7} %{buildroot}%{_cross_libdir}/bottlerocket/ephemeral-storage.d/docker.conf

%cross_scan_attribution --clarify %{S:1000} go-vendor vendor

%files
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%{_cross_unitdir}/docker.service
%{_cross_unitdir}/docker.socket
%{_cross_unitdir}/prepare-var-lib-docker.service
%{_cross_sysusersdir}/docker.conf
%{_cross_templatedir}/docker-daemon-json
%{_cross_templatedir}/docker-daemon-nvidia-json
%{_cross_tmpfilesdir}/docker-engine.conf
%{_cross_libdir}/bottlerocket/ephemeral-storage.d/docker.conf
%{_cross_bindir}/dockerd
%{_cross_bindir}/docker-proxy

%changelog

