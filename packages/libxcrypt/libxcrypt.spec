Name: %{_cross_os}libxcrypt
Version: 4.4.38
Release: 1%{?dist}
Summary: Extended crypt library for descrypt, md5crypt, bcrypt, and others
License: LGPL-2.1-or-later
URL: https://github.com/besser82/libxcrypt
Source0: https://github.com/besser82/libxcrypt/releases/download/v%{version}/libxcrypt-%{version}.tar.xz
Source1: https://github.com/besser82/libxcrypt/releases/download/v%{version}/libxcrypt-%{version}.tar.xz.asc
Source2: gpgkey-678CE3FEE430311596DB8C16F52E98007594C21D.asc
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package devel
Summary: Files for development using the extended crypt library for descrypt, md5crypt, bcrypt, and others
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n libxcrypt-%{version} -p1

%build
%cross_configure \
  --disable-failure-tokens \
  --disable-valgrind \
  --disable-silent-rules \
  --enable-hashes=all \
  --enable-obsolete-api=no \
  --enable-obsolete-api-enosys=no \
  --enable-shared \
  --enable-static \
  --with-pkgconfigdir=%{_cross_pkgconfigdir} \

%make_build
%cross_generate_sbom

%install
%make_install
%cross_install_sbom

%files
%license LICENSING COPYING.LIB
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%{_cross_libdir}/*.so.*
%exclude %{_cross_mandir}

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.so
%{_cross_includedir}/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
