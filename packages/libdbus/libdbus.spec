Name: %{_cross_os}libdbus
Version: 1.15.12
Release: 1%{?dist}
Epoch: 1
Summary: Library for a message bus
License: AFL-2.1 OR GPL-2.0-or-later
URL: http://www.freedesktop.org/Software/dbus/
Source0: https://dbus.freedesktop.org/releases/dbus/dbus-%{version}.tar.xz
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libcap-devel
BuildRequires: %{_cross_os}libexpat-devel
Requires: %{_cross_os}libcap
Requires: %{_cross_os}libexpat

%description
%{summary}.

%package devel
Summary: Files for development using the library for a message bus
Requires: %{name}

%description devel
%{summary}.

%prep
%autosetup -n dbus-%{version} -p1

%build
CONFIGURE_OPTS=(
 -Dasserts=false
 -Dcontainers=false
 -Dembedded_tests=false
 -Dinstalled_tests=false
 -Dmessage_bus=false
 -Dstats=false
 -Dtools=false
 -Dtraditional_activation=false
 -Duser_session=false

 -Dapparmor=disabled
 -Ddoxygen_docs=disabled
 -Dducktype_docs=disabled
 -Dkqueue=disabled
 -Dlaunchd=disabled
 -Dlibaudit=disabled
 -Dmodular_tests=disabled
 -Dqt_help=disabled
 -Drelocation=disabled
 -Dselinux=disabled
 -Dsystemd=disabled
 -Dvalgrind=disabled
 -Dx11_autolaunch=disabled
 -Dxml_docs=disabled

 -Dchecks=true
)

%cross_meson "${CONFIGURE_OPTS[@]}"
%cross_meson_build

%install
%cross_meson_install

rm -rf %{buildroot}%{_cross_docdir}/dbus/examples

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*
%exclude %{_cross_datadir}/doc
%exclude %{_cross_datadir}/xml

%files devel
%{_cross_libdir}/*.so
%dir %{_cross_libdir}/dbus-1.0
%{_cross_libdir}/dbus-1.0/*
%dir %{_cross_includedir}/dbus-1.0
%{_cross_includedir}/dbus-1.0/*
%{_cross_pkgconfigdir}/*.pc
%exclude %{_cross_libdir}/*.la
%exclude %{_cross_libdir}/cmake

%changelog
