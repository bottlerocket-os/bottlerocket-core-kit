Name: %{_cross_os}libsemanage
Version: 3.8.1
Release: 1%{?dist}
Epoch: 1
Summary: Library for SELinux binary policy manipulation
License: LGPL-2.1-or-later
URL: https://github.com/SELinuxProject/
Source0: https://github.com/SELinuxProject/selinux/releases/download/%{version}/libsemanage-%{version}.tar.gz
Source1: https://github.com/SELinuxProject/selinux/releases/download/%{version}/libsemanage-%{version}.tar.gz.asc
Source2: gpgkey-7200EB2C3F5E488463C0CE9ECDCAE8C927C6BE31.asc
Patch0001: 0001-remove-bzip2-dependency.patch

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libaudit-devel
BuildRequires: %{_cross_os}libselinux-devel
BuildRequires: %{_cross_os}libsepol-devel
Requires: %{_cross_os}libaudit
Requires: %{_cross_os}libselinux
Requires: %{_cross_os}libsepol

%description
%{summary}.

%package devel
Summary: Files for development using the library for SELinux binary policy manipulation
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n libsemanage-%{version} -p1

%global set_env \
%set_cross_build_flags \\\
export CC="%{_cross_target}-gcc" \\\
export DESTDIR='%{buildroot}' \\\
export PREFIX='%{_cross_prefix}' \\\
%{nil}

%build
%set_env
%make_build
%cross_generate_sbom

%install
%set_env
%make_install
%cross_install_sbom

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_libdir}/*.so.*
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%exclude %{_cross_libexecdir}
%exclude %{_cross_mandir}
%exclude %{_cross_sysconfdir}

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.so
%dir %{_cross_includedir}/semanage
%{_cross_includedir}/semanage/*
%{_cross_pkgconfigdir}/*.pc

%changelog
