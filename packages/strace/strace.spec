Name: %{_cross_os}strace
Version: 6.15
Release: 1%{?dist}
Summary: Linux syscall tracer
License: LGPL-2.1-or-later
URL: https://strace.io/
Source0: https://strace.io/files/%{version}/strace-%{version}.tar.xz
Source1: https://strace.io/files/%{version}/strace-%{version}.tar.xz.asc
Source2: gpgkey-296D6F29A020808E8717A8842DB5BD89A340AEB7.asc
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n strace-%{version} -p1

%build
%cross_configure \
  --disable-mpers \

%make_build
%cross_generate_sbom

%install
%make_install
%cross_install_sbom

%files
%license COPYING LGPL-2.1-or-later
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%{_cross_bindir}/strace
%exclude %{_cross_bindir}/strace-log-merge
%exclude %{_cross_mandir}/*

%changelog
