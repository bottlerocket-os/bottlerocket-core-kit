Name: %{_cross_os}grep
Version: 3.12
Release: 1%{?dist}
Epoch: 1
Summary: GNU grep utility
URL: https://www.gnu.org/software/grep/
License: GPL-3.0-or-later
Source0: https://mirrors.kernel.org/gnu/grep/grep-%{version}.tar.xz
Source1: https://mirrors.kernel.org/gnu/grep/grep-%{version}.tar.xz.sig
Source2: gpgkey-155D3FC500C834486D1EEA677FD9FCCB000BEEEE.asc
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libpcre-devel
Requires: %{_cross_os}libpcre

%description
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n grep-%{version} -p1

%build
%cross_configure --without-included-regex --disable-silent-rules
%make_build
%cross_generate_sbom

%install
%make_install
%cross_install_sbom

%files
%license COPYING
%{_cross_bindir}/grep
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
# Exclude fgrep and egrep because they are shell scripts
%exclude %{_cross_bindir}/fgrep
%exclude %{_cross_bindir}/egrep
%exclude %{_cross_infodir}
%exclude %{_cross_localedir}
%exclude %{_cross_mandir}
