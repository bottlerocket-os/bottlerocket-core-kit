%global project moby
%global repo github.com/moby/%{project}
%global goorg github.com/moby
%global goimport %{goorg}/moby

%global gover 29.0.0-rc.2
%global rpmver 29.0.0
%global gitrev bb45a3f4a0eaaa3afe8145acc5a901fcad717417

%global source_date_epoch 1363394400

%global _dwz_low_mem_die_limit 0

%global package_priority_epoch 0

Name: %{_cross_os}docker-engine-29
Version: %{rpmver}
Release: 0.rc2%{?dist}
Summary: Docker engine
License: Apache-2.0
URL: https://%{repo}
Source0: https://%{repo}/archive/docker-v%{gover}/%{project}-docker-v%{gover}.tar.gz
Source1: docker.service
Source2: docker.socket
Source3: docker-sysusers.conf
Source4: daemon-json
Source5: daemon-nvidia-json

# Create container storage mount point.
Source100: prepare-var-lib-docker.service

Source1000: clarify.toml

Patch0001: 0001-Change-default-capabilities-using-daemon-config.patch
Patch0002: 0002-oci-inject-kmod-in-all-containers.patch
Patch0003: 0003-Switch-containerd-image-backend-s-image-pull-to-tran.patch

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
Requires: %{name}(binaries)

Provides: %{_cross_os}docker-engine = %{package_priority_epoch}:
Conflicts: %{_cross_os}docker-engine

%description
%{summary}.

%package bin
Summary: Docker engine binaries
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(no-fips) and %{name})
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-fips-bin)

%description bin
%{summary}.

%package fips-bin
Summary: Docker engine binaries, FIPS edition
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-bin)

%description fips-bin
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

gofips build "${BUILD_ARGS[@]}" -o fips/dockerd %{goimport}/v2/cmd/dockerd
gofips build "${BUILD_ARGS[@]}" -o fips/docker-proxy %{goimport}/v2/cmd/docker-proxy

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 dockerd %{buildroot}%{_cross_bindir}
install -p -m 0755 docker-proxy %{buildroot}%{_cross_bindir}

install -d %{buildroot}%{_cross_fips_bindir}
install -p -m 0755 fips/dockerd %{buildroot}%{_cross_fips_bindir}
install -p -m 0755 fips/docker-proxy %{buildroot}%{_cross_fips_bindir}

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:1} %{S:100} %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:2} %{buildroot}%{_cross_unitdir}/docker.socket

install -d %{buildroot}%{_cross_sysusersdir}
install -p -m 0644 %{S:3} %{buildroot}%{_cross_sysusersdir}/docker.conf

install -d %{buildroot}%{_cross_templatedir}
install -p -m 0644 %{S:4} %{buildroot}%{_cross_templatedir}/docker-daemon-json
install -p -m 0644 %{S:5} %{buildroot}%{_cross_templatedir}/docker-daemon-nvidia-json

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

%files bin
%{_cross_bindir}/dockerd
%{_cross_bindir}/docker-proxy

%files fips-bin
%{_cross_fips_bindir}/dockerd
%{_cross_fips_bindir}/docker-proxy

%changelog
