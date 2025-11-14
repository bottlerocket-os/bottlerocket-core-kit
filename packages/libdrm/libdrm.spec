Name: %{_cross_os}libdrm
Version: 2.4.129
Release: 1%{?dist}
Summary: Direct rendering manager library
License: MIT
URL: https://dri.freedesktop.org
Source0: https://dri.freedesktop.org/libdrm/libdrm-%{version}.tar.xz
Source1: https://dri.freedesktop.org/libdrm/libdrm-%{version}.tar.xz.sig
Source2: gpgkey-34FF9526CFEF0E97A340E2E40FDE7BE0E88F5E48.asc

BuildRequires: %{_cross_os}glibc-devel
Requires: %{_cross_os}glibc

%description
%{summary}.

%package devel
Summary: Files for development using the direct rendering manager library
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n libdrm-%{version} -p1

%build
CONFIGURE_OPTS=(
 --auto-features=disabled
 -Dcairo-tests=disabled
 -Dman-pages=disabled
 -Dvalgrind=disabled
 -Dfreedreno=disabled
 -Dvc4=disabled
 -Detnaviv=disabled
 -Dexynos=disabled
 -Dtegra=disabled
 -Domap=disabled
 -Dintel=disabled
 -Dradeon=disabled
 -Damdgpu=enabled
 -Dnouveau=disabled
 -Dtests=false
)

%cross_meson "${CONFIGURE_OPTS[@]}"
%cross_meson_build

%install
%cross_meson_install

%files
%{_cross_attribution_file}
%{_cross_libdir}/libdrm.so.*
%{_cross_libdir}/libdrm_amdgpu.so.*
%{_cross_datadir}/libdrm/amdgpu.ids

%files devel
%{_cross_libdir}/libdrm.so
%{_cross_libdir}/libdrm_amdgpu.so
%{_cross_includedir}/*
%{_cross_pkgconfigdir}/*.pc
