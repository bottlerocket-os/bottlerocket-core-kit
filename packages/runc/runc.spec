%global goproject github.com/opencontainers
%global gorepo runc
%global goimport %{goproject}/%{gorepo}
%global commit eeb7e6024f9ee43876301b1d23c353384fa6dcdd
%global gover 1.2.8

%global _dwz_low_mem_die_limit 0

Name: %{_cross_os}%{gorepo}
Version: %{gover}
Release: 1%{?dist}
Epoch: 1
Summary: CLI for running Open Containers
License: Apache-2.0
URL: https://%{goimport}
Source0: https://%{goimport}/releases/download/v%{gover}/%{gorepo}.tar.xz#/%{gorepo}-v%{gover}.tar.xz
Source1: https://%{goimport}/releases/download/v%{gover}/%{gorepo}.tar.xz.asc#/%{gorepo}-v%{gover}.tar.xz.asc
# Aleksa Sarai
Source2: gpgkey-B64E4955B29FA3D463F2A9062897FAD2B7E9446F.asc
# Kir Kolyshkin
Source3: gpgkey-C2428CD75720FACDCF76B6EA17DE5ECB75A1100E.asc

Patch1001: 1001-rootfs-only-set-mode-for-tmpfs-mount.patch

BuildRequires: git
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libseccomp-devel
Requires: %{_cross_os}libseccomp
Requires: %{name}(binaries)

%description
%{summary}.

%package bin
Summary: CLI for running Open Containers binaries
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(no-fips) and %{name})
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-fips-bin)

%description bin
%{summary}.

%package fips-bin
Summary: CLI for running Open Containers binaries, FIPS edition
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-bin)

%description fips-bin
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -Sgit -n %{gorepo}-%{gover} -p1
%cross_go_setup %{gorepo}-%{gover} %{goproject} %{goimport}

%build
%cross_go_configure %{goimport}
export LD_VERSION="-X main.version=%{gover}+bottlerocket"
export LD_COMMIT="-X main.gitCommit=%{commit}"
export BUILDTAGS="ambient seccomp selinux"

declare -a BUILD_ARGS
BUILD_ARGS=(
  -ldflags="${GOLDFLAGS} ${LD_VERSION} ${LD_COMMIT}"
  -tags="${BUILDTAGS}"
)

go build "${BUILD_ARGS[@]}" -o bin/runc .
gofips build "${BUILD_ARGS[@]}" -o fips/bin/runc .

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 bin/runc %{buildroot}%{_cross_bindir}

install -d %{buildroot}%{_cross_fips_bindir}
install -p -m 0755 fips/bin/runc %{buildroot}%{_cross_fips_bindir}

%cross_scan_attribution go-vendor vendor

%files
%license LICENSE NOTICE
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}

%files bin
%{_cross_bindir}/runc

%files fips-bin
%{_cross_fips_bindir}/runc

%changelog
