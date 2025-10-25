# Skip check-rpaths since we expect them for systemd.
%global __brp_check_rpaths %{nil}

Name: %{_cross_os}libudev
Version: 252.39
Release: 1%{?dist}
Summary: System and Service Manager
License: GPL-2.0-or-later AND GPL-2.0-only AND LGPL-2.1-or-later
URL: https://www.freedesktop.org/wiki/Software/systemd
Source0: https://github.com/systemd/systemd-stable/archive/v%{version}/systemd-stable-%{version}.tar.gz

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}kmod-devel
BuildRequires: %{_cross_os}libacl-devel
BuildRequires: %{_cross_os}libattr-devel
BuildRequires: %{_cross_os}libblkid-devel
BuildRequires: %{_cross_os}libcap-devel
BuildRequires: %{_cross_os}libfdisk-devel
BuildRequires: %{_cross_os}libmount-devel
BuildRequires: %{_cross_os}libseccomp-devel
BuildRequires: %{_cross_os}libselinux-devel
BuildRequires: %{_cross_os}libuuid-devel
BuildRequires: %{_cross_os}libxcrypt-devel

# Discourage dnf from picking this bootstrap package during image builds.
Conflicts: %{_cross_os}filesystem
Conflicts: %{_cross_os}release
Conflicts: %{_cross_os}systemd

%description
%{summary}.

%package devel
Summary: Files for development using the System and Service Manager
Requires: %{name}
Requires: %{_cross_os}libcap

%description devel
%{summary}.

%prep
%autosetup -n systemd-stable-%{version} -p1

%build
CONFIGURE_OPTS=(
 -Dmode=release

 -Drootprefix='%{_cross_prefix}'
 -Drootlibdir='%{_cross_libdir}'

 -Dpkgconfigdatadir='%{_cross_pkgconfigdir}'
 -Dpkgconfiglibdir='%{_cross_pkgconfigdir}'
)

%cross_meson "${CONFIGURE_OPTS[@]}"
%cross_meson_build

%install
%cross_meson_install

find %{buildroot} \( -type f -o -type l \) ! -name '*libudev*' -delete

%files
%license LICENSE.GPL2 LICENSE.LGPL2.1
%{_cross_attribution_file}
%{_cross_libdir}/libudev.so.*

%files devel
%{_cross_libdir}/libudev.so
%{_cross_includedir}/libudev.h
%{_cross_pkgconfigdir}/libudev*.pc
