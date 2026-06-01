%global gorepo soci-snapshotter
%global gover 0.13.0
%global rpmver %{gover}
%global gitrev e5a21c6772046d6bc0366666a78b0286260284e4

Name: %{_cross_os}soci-snapshotter
Version: %{gover}
Release: 1%{?dist}
Epoch: 1
Summary: A containerd snapshotter plugin which enables lazy loading for OCI images.
License: Apache-2.0
URL: https://github.com/awslabs/soci-snapshotter
Source0: https://github.com/awslabs/soci-snapshotter/archive/v%{gover}/soci-snapshotter-%{gover}.tar.gz
Source1: bundled-soci-snapshotter-%{gover}.tar.gz
Source2: bundled-cmd.tar.gz
Source3: soci-config-toml
Source4: k8s-snapshotter-conf
Source100: etc-soci-snapshotter.mount.in
Source101: soci-snapshotter.service
Source102: soci-snapshotter.socket
Source1000: clarify.toml

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libz-devel
Requires: (%{name}-k8s if %{_cross_os}variant-runtime(k8s))
Requires: %{name}(optimized-gunzip)

%description
%{summary}.

%package pigz
Summary: Prefer pigz for gzip decompression
Requires: %{_cross_os}pigz
Requires: %{name}
Provides: %{name}(optimized-gunzip) = 1:
Conflicts: %{name}-igzip

%description pigz
%{summary}.

%package igzip
Summary: Prefer igzip for gzip decompression
Requires: %{_cross_os}igzip
Requires: %{name}
Conflicts: %{name}-pigz
%if "%{_cross_arch}" == "x86_64"
Provides: %{name}(optimized-gunzip) = 2:
%else
Provides: %{name}(optimized-gunzip) = 0:
%endif

%description igzip
%{summary}.

%package k8s
Summary: Drop-ins to override the kubelet's configuration
Provides: %{name}(k8s)

%description k8s
%{summary}.

%prep
%autosetup -n %{gorepo}-%{gover} -p1
%setup -T -D -n %{gorepo}-%{gover} -b 1 -q
%setup -T -D -n %{gorepo}-%{gover} -b 2 -q

%build
%set_cross_go_flags

export LD_VERSION="-X github.com/awslabs/soci-snapshotter/version.Version=v%{gover}+bottlerocket"
export LD_REVISION="-X github.com/awslabs/soci-snapshotter/version.Revision=%{gitrev}"

go build -C cmd -ldflags="${GOLDFLAGS} ${LD_VERSION} ${LD_REVISION}" -o "../out/soci-snapshotter-grpc" ./soci-snapshotter-grpc

%install
install -d %{buildroot}%{_cross_bindir}
install -d %{buildroot}%{_cross_unitdir}
install -p -m 0755 out/soci-snapshotter-grpc %{buildroot}%{_cross_bindir}

SOCIMOUNTPATH=$(systemd-escape --path /etc/soci-snapshotter)
install -p -m 0644 %{S:100} %{buildroot}%{_cross_unitdir}/${SOCIMOUNTPATH}.mount

install -D -p -m 0644 %{S:101} %{buildroot}%{_cross_unitdir}
install -D -p -m 0644 %{S:102} %{buildroot}%{_cross_unitdir}

install -d %{buildroot}%{_cross_templatedir}
install -p -m 0644 %{S:3} %{buildroot}%{_cross_templatedir}/soci-config-toml
install -p -m 0644 %{S:4} %{buildroot}%{_cross_templatedir}/k8s-snapshotter-conf

%cross_scan_attribution --clarify %{S:1000} go-vendor vendor

%post igzip -p <lua>
posix.symlink("%{_cross_bindir}/igzip", "%{_cross_bindir}/soci-gunzip")

%post pigz -p <lua>
posix.symlink("%{_cross_bindir}/unpigz", "%{_cross_bindir}/soci-gunzip")

%files
%license LICENSE NOTICE.md
%{_cross_unitdir}/soci-snapshotter.service
%{_cross_unitdir}/soci-snapshotter.socket
%{_cross_unitdir}/etc-soci\x2dsnapshotter.mount
%{_cross_attribution_vendor_dir}
%{_cross_attribution_file}
%{_cross_templatedir}/soci-config-toml
%{_cross_bindir}/soci-snapshotter-grpc

%files pigz
# No files provided by pigz but required for packaging.

%files igzip
# No files provided by igzip but required for packaging.

%files k8s
%{_cross_templatedir}/k8s-snapshotter-conf

%changelog
