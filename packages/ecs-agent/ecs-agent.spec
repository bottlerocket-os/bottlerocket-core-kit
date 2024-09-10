%global agent_goproject github.com/aws
%global agent_gorepo amazon-ecs-agent
%global agent_goimport %{agent_goproject}/%{agent_gorepo}

%global agent_gover 1.82.3

# git rev-parse --short=8
%global agent_gitrev b7e96508

# Construct reproducible tar archives
# See https://reproducible-builds.org/docs/archives/
%global source_date_epoch 1234567890
%global tar_cf tar --sort=name --mtime="@%{source_date_epoch}" --owner=0 --group=0 --numeric-owner -cf

Name: %{_cross_os}ecs-agent
Version: %{agent_gover}
Release: 1%{?dist}
Summary: Amazon Elastic Container Service agent
License: Apache-2.0
URL: https://%{agent_goimport}
Source0: https://%{agent_goimport}/archive/v%{agent_gover}/%{agent_gorepo}-%{agent_gover}.tar.gz
Source101: ecs.service
Source102: ecs-tmpfiles.conf
Source103: ecs-sysctl.conf
Source104: ecs-base-conf
Source105: pause-image-VERSION
Source106: pause-config.json
Source107: pause-manifest.json
Source108: pause-repositories
# Bottlerocket-specific - version data can be set with linker options
Source109: version.go
Source110: ecs-defaults.conf
Source111: ecs-nvidia.conf

# Mount for writing ECS agent configuration
Source200: etc-systemd-system-ecs.service.d.mount

# Ecs logdog configuration
Source300: logdog.ecs.conf

# Bottlerocket-specific - filesystem location of the pause image
Patch0001: 0001-bottlerocket-default-filesystem-locations.patch

# Bottlerocket-specific - remove unsupported capabilities
Patch0002: 0002-bottlerocket-remove-unsupported-capabilities.patch

# bind introspection to localhost
# https://github.com/aws/amazon-ecs-agent/pull/2588
Patch0003: 0003-bottlerocket-bind-introspection-to-localhost.patch

# Bottlerocket-specific - fix procfs path for non-containerized ECS agent
Patch0004: 0004-bottlerocket-fix-procfs-path-on-host.patch

# Bottlerocket-specific - fix ECS exec directories
Patch0005: 0005-bottlerocket-change-execcmd-directories-for-Bottlero.patch

# Bottlerocket-specific - fix container metadata path
Patch0006: 0006-containermetadata-don-t-use-dataDirOnHost-for-metada.patch

BuildRequires: %{_cross_os}glibc-devel

Requires: %{_cross_os}docker-engine
Requires: %{_cross_os}iptables
Requires: %{_cross_os}amazon-ssm-agent-plugin
Requires: %{_cross_os}amazon-ecs-cni-plugins
Requires: %{_cross_os}amazon-vpc-cni-plugins
Requires: %{name}(binaries)

%description
%{summary}.

%package bin
Summary: Amazon Elastic Container Service agent binaries
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(no-fips) and %{name})
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-fips-bin)

%description bin
%{summary}.

%package fips-bin
Summary: Amazon Elastic Container Service agent binaries, FIPS edition
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-bin)

%description fips-bin
%{summary}.

%package config
Summary: Base configuration files for the ECS agent
Requires: %{name}

%description config
%{summary}.

%package nvidia-config
Summary: NVIDIA specific configuration files for the ECS agent
Requires: %{name}
Requires: %{name}-config

%description nvidia-config
%{summary}.

%prep
%autosetup -n %{agent_gorepo}-%{agent_gover} -p1
# Replace upstream's version.go to support build-time values from ldflags. This
# avoids maintenance of patches that use always changing version-control tokens
# in its replacement.
cp %{S:109} "agent/version/version.go"

# Symlink amazon-ecs-agent-%{agent_gover} to the GOPATH location
%cross_go_setup %{agent_gorepo}-%{agent_gover} %{agent_goproject} %{agent_goimport}

%build
# cross_go_configure cd's to the correct GOPATH location
%cross_go_configure %{agent_goimport}
PAUSE_CONTAINER_IMAGE_NAME="amazon/amazon-ecs-pause"
PAUSE_CONTAINER_IMAGE_TAG="bottlerocket"
LD_PAUSE_CONTAINER_NAME="-X github.com/aws/amazon-ecs-agent/agent/config.DefaultPauseContainerImageName=${PAUSE_CONTAINER_IMAGE_NAME}"
LD_PAUSE_CONTAINER_TAG="-X github.com/aws/amazon-ecs-agent/agent/config.DefaultPauseContainerTag=${PAUSE_CONTAINER_IMAGE_TAG}"
LD_VERSION="-X github.com/aws/amazon-ecs-agent/agent/version.Version=%{agent_gover}"
LD_GIT_REV="-X github.com/aws/amazon-ecs-agent/agent/version.GitShortHash=%{agent_gitrev}"

declare -a ECS_AGENT_BUILD_ARGS
ECS_AGENT_BUILD_ARGS=(
  -ldflags "${GOLDFLAGS} ${LD_PAUSE_CONTAINER_NAME} ${LD_PAUSE_CONTAINER_TAG} ${LD_VERSION} ${LD_GIT_REV}"
)

go build "${ECS_AGENT_BUILD_ARGS[@]}" -o amazon-ecs-agent ./agent
gofips build "${ECS_AGENT_BUILD_ARGS[@]}" -o fips/amazon-ecs-agent ./agent

# Build the pause container
(
  set -x
  cd misc/pause-container/

  # Build static pause executable for container image.
  mkdir -p rootfs/usr/bin
  %{_cross_triple}-musl-gcc ${_cross_cflags} -static pause.c -o rootfs/usr/bin/pause

  # Construct container image.
  mkdir -p image/rootfs
  %tar_cf image/rootfs/layer.tar -C rootfs .
  DIGEST=$(sha256sum image/rootfs/layer.tar | sed -e 's/ .*//')
  install -m 0644 %{S:105} image/rootfs/VERSION
  install -m 0644 %{S:106} image/config.json
  sed -i "s/~~digest~~/${DIGEST}/" image/config.json
  install -m 0644 %{S:107} image/manifest.json
  install -m 0644 %{S:108} image/repositories
  %tar_cf ../../amazon-ecs-pause.tar -C image .
)

%install
install -d %{buildroot}%{_cross_bindir}
install -D -p -m 0755 amazon-ecs-agent %{buildroot}%{_cross_bindir}/amazon-ecs-agent
install -D -p -m 0644 amazon-ecs-pause.tar %{buildroot}%{_cross_libdir}/amazon-ecs-agent/amazon-ecs-pause.tar

install -d %{buildroot}%{_cross_fips_bindir}
install -D -p -m 0755 fips/amazon-ecs-agent %{buildroot}%{_cross_fips_bindir}/amazon-ecs-agent

install -d %{buildroot}%{_cross_unitdir}
install -D -p -m 0644 %{S:101} %{S:200} %{buildroot}%{_cross_unitdir}

install -d %{buildroot}%{_cross_unitdir}/ecs.service.d/
install -D -p -m 0644 %{S:110} %{buildroot}%{_cross_unitdir}/ecs.service.d/00-defaults.conf
install -D -p -m 0644 %{S:111} %{buildroot}%{_cross_unitdir}/ecs.service.d/20-nvidia.conf

install -D -p -m 0644 %{S:102} %{buildroot}%{_cross_tmpfilesdir}/ecs.conf
install -D -p -m 0644 %{S:103} %{buildroot}%{_cross_sysctldir}/90-ecs.conf

install -D -p -m 0644 %{S:104} %{buildroot}%{_cross_templatedir}/ecs-base-conf

# Directory for agents used by the ECS agent, e.g. SSM, Service Connect
%global managed_agents %{_cross_libexecdir}/amazon-ecs-agent/managed-agents
install -d %{buildroot}%{managed_agents}

# Directory for ECS exec artifacts
%global ecs_exec_dir %{managed_agents}/execute-command
install -d %{buildroot}%{ecs_exec_dir}

# The ECS agent looks for real versioned directories under bin, symlinks will be
# ignored. Thus, link the bin directory in the ssm-agent directory which contains
# the versioned binaries.
ln -rs %{buildroot}%{_cross_libexecdir}/amazon-ssm-agent/bin %{buildroot}/%{ecs_exec_dir}/bin

# The ECS agent generates and stores configurations for ECS exec sessions inside
# "config", thus reference it with a symlink to a directory under /var
ln -rs %{buildroot}%{_cross_localstatedir}/ecs/managed-agents/execute-command/config %{buildroot}%{ecs_exec_dir}/config

# Use the host's certificates bundle for ECS exec sessions
install -d %{buildroot}%{ecs_exec_dir}/certs
ln -rs %{buildroot}%{_cross_sysconfdir}/pki/tls/certs/ca-bundle.crt %{buildroot}%{ecs_exec_dir}/certs/tls-ca-bundle.pem

%cross_scan_attribution go-vendor agent/vendor

install -d %{buildroot}%{_cross_datadir}/logdog.d
install -p -m 0644 %{S:300} %{buildroot}%{_cross_datadir}/logdog.d

%files
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%license LICENSE NOTICE ecs-agent/THIRD_PARTY.md

%{_cross_libexecdir}/amazon-ecs-agent/managed-agents
%{_cross_unitdir}/ecs.service
%{_cross_unitdir}/etc-systemd-system-ecs.service.d.mount
%{_cross_tmpfilesdir}/ecs.conf
%{_cross_sysctldir}/90-ecs.conf
%{_cross_libdir}/amazon-ecs-agent/amazon-ecs-pause.tar
%{_cross_datadir}/logdog.d/logdog.ecs.conf

%files bin
%{_cross_bindir}/amazon-ecs-agent

%files fips-bin
%{_cross_fips_bindir}/amazon-ecs-agent

%files config
%{_cross_templatedir}/ecs-base-conf
%{_cross_unitdir}/ecs.service.d/00-defaults.conf

%files nvidia-config
%{_cross_unitdir}/ecs.service.d/20-nvidia.conf

%changelog
