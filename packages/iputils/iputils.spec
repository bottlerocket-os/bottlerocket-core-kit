Name: %{_cross_os}iputils
Version: 20250605
Release: 1%{?dist}
Summary: A set of network monitoring tools
License: GPL-2.0-or-later AND BSD-3-Clause
URL: https://github.com/iputils/iputils
Source0: https://github.com/iputils/iputils/releases/download/%{version}/iputils-%{version}.tar.gz
Source1: https://github.com/iputils/iputils/releases/download/%{version}/iputils-%{version}.tar.gz.asc
Source2: gpgkey-2016FEA4858B1C36B32E833AC0DEC2EE72F33A5F.asc

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libcap-devel
Requires: %{_cross_os}libcap

%description
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n iputils-%{version} -p1

%build
CONFIGURE_OPTS=(
 -DUSE_CAP=true
 -DUSE_GETTEXT=false
 -DUSE_IDN=false

 -DBUILD_ARPING=true
 -DBUILD_PING=true
 -DBUILD_TRACEPATH=true

 -DBUILD_CLOCKDIFF=false

 -DBUILD_MANS=false
 -DBUILD_HTML_MANS=false
)

%cross_meson "${CONFIGURE_OPTS[@]}"
%cross_meson_build
%cross_generate_sbom

%install
%cross_meson_install
%cross_install_sbom

%files
%license LICENSE Documentation/LICENSE.GPL2 Documentation/LICENSE.BSD3
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%attr(0755,root,root) %caps(cap_net_raw=p) %{_cross_bindir}/arping
%attr(0755,root,root) %caps(cap_net_raw=p cap_net_admin=p) %{_cross_bindir}/ping
%{_cross_bindir}/tracepath

%changelog
