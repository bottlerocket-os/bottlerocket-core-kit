Name: %{_cross_os}pigz
Version: 2.8
Release: 1%{?dist}
Epoch: 1
Summary: pigz is a parallel implementation of gzip which utilizes multiple cores
License: Zlib AND Apache-2.0
URL: http://www.zlib.net/pigz
Source0: https://zlib.net/pigz/pigz-%{version}.tar.gz
Source1: https://zlib.net/pigz/pigz-%{version}-sig.txt
Source2: gpgkey-5ED46A6721D365587791E2AA783FCD8E58BCAFBA.asc
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libz-devel

%description
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n pigz-%{version} -p1

%global set_env \
%set_cross_build_flags \\\
export CC=%{_cross_target}-gcc \\\
%{nil}

%build
%set_env
%make_build CC="${CC}" CFLAGS="${CFLAGS}" LDFLAGS="${LDFLAGS}"
%cross_generate_sbom

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 unpigz %{buildroot}%{_cross_bindir}
%cross_install_sbom

%files
%license README zopfli/COPYING
%{_cross_bindir}/unpigz
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json

%changelog
