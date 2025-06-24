Name: %{_cross_os}libsepol
Version: 3.8.1
Release: 1%{?dist}
Epoch: 1
Summary: Library for SELinux policy manipulation
License: LGPL-2.1-or-later
URL: https://github.com/SELinuxProject/
Source0: https://github.com/SELinuxProject/selinux/releases/download/%{version}/libsepol-%{version}.tar.gz
Source1: https://github.com/SELinuxProject/selinux/releases/download/%{version}/libsepol-%{version}.tar.gz.asc
Source2: gpgkey-7200EB2C3F5E488463C0CE9ECDCAE8C927C6BE31.asc
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the library for SELinux policy manipulation
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n libsepol-%{version} -p1

%global set_env \
%set_cross_build_flags \\\
export CC="%{_cross_target}-gcc" \\\
export DESTDIR='%{buildroot}' \\\
export PREFIX='%{_cross_prefix}' \\\
export SHLIBDIR='%{_cross_libdir}' \\\
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
%exclude %{_cross_bindir}
%exclude %{_cross_mandir}

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.so
%dir %{_cross_includedir}/sepol
%{_cross_includedir}/sepol/*
%{_cross_pkgconfigdir}/*.pc

%changelog
