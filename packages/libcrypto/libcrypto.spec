# Disable LTO since the performance critical code is all written in
# assembly, and optimizations applied to the C code could affect FIPS
# or overall correctness.
%global _cross_cflags %{_cross_cflags} -fno-lto
%global _cross_cxxflags %{_cross_cflags}

Name: %{_cross_os}libcrypto
Version: 3.3.0
Release: 1%{?dist}
Summary: AWS-LC cryptographic library
License: ISC AND (Apache-2.0 OR ISC) AND OpenSSL
URL: https://github.com/aws/aws-lc

Source0: https://github.com/aws/aws-lc/archive/AWS-LC-FIPS-%{version}/aws-lc-AWS-LC-FIPS-%{version}.tar.gz

# Upstream patches from AWS-LC-FIPS 3.3 branch can be fetched using the script
# at generate-aws-lc-patches.sh

Patch1019: 1019-Cherry-pick-2024-Implement-SSL_CTX_set_client_hello_.patch
Patch1020: 1020-FIPS-2024-CHERRYPICK-Fix-shared-library-install-on-W.patch
Patch1021: 1021-FIPS-2024-CHERRY-PICK-Generate-Rust-Bindings-2999-32.patch
Patch1022: 1022-CHERRYPICK-FIPS-3.x-Make-rustfmt-optional-for-Rust-b.patch
Patch1023: 1023-CHERRYPICK-FIPS-3.x-Support-SSL_OP_IGNORE_UNEXPECTED.patch
Patch1024: 1024-Prepare-v3.4.0-3306.patch
Patch1025: 1025-CHERRYPICK-FIPS-3.0-Fix-SSL_OP_IGNORE_UNEXPECTED_EOF.patch
Patch1026: 1026-Prepare-v3.5.0-3362.patch
Patch1027: 1027-Cherry-pick-2024-Enable-Hybrid-PQ-KeyShares-by-defau.patch
Patch1028: 1028-Cherry-pick-2024-Fix-CMake-Compatability-CI-jobs-295.patch
Patch9001: 9001-fix-memchr-const-correctness-for-C23-compatibility-as-implemented-in-glibc-2.43.patch

BuildRequires: %{_cross_os}glibc-devel
Requires: %{_cross_os}glibc

%description
%{summary}.

%package -n %{_cross_os}libssl
Summary: OpenSSL shim for the AWS-LC cryptographic library
Provides: %{_cross_os}openssl
Requires: %{name}

%description -n %{_cross_os}libssl
%{summary}.

%package tools
Summary: Command line tools for the AWS-LC cryptographic library
Provides: %{_cross_os}openssl-tools
Requires: %{name}
Requires: %{_cross_os}libssl

%description tools
%{summary}.

%package devel
Summary: Files for development using the AWS-LC cryptographic library
Requires: %{name}

%description devel
%{summary}.

%package -n %{_cross_os}libssl-devel
Summary: Files for development using the OpenSSL shim for the AWS-LC cryptographic library
Provides: %{_cross_os}openssl-devel
Requires: %{_cross_os}libssl-devel

%description -n %{_cross_os}libssl-devel
%{summary}.

%prep
%autosetup -S git -n aws-lc-AWS-LC-FIPS-%{version} -p1

%build
%cross_cmake \
  -GNinja \
  -DCMAKE_BUILD_TYPE=RelWithDebInfo \
  -DBUILD_SHARED_LIBS=ON \
  -DBUILD_TESTING=OFF \
  -DCMAKE_INSTALL_PREFIX=%{_cross_prefix} \
  -DCMAKE_INSTALL_LIBDIR=%{_cross_libdir} \
  -DCMAKE_SKIP_INSTALL_RPATH=ON \
  -DFIPS=1 \
  %{nil}

%ninja_build

%install
%ninja_install

%files
%license LICENSE NOTICE
%{_cross_attribution_file}
%{_cross_libdir}/libcrypto.so

%files -n %{_cross_os}libssl
%{_cross_libdir}/libssl.so

%files tools
%{_cross_bindir}/bssl
%{_cross_bindir}/openssl

%files devel
%{_cross_includedir}/openssl
%{_cross_pkgconfigdir}/libcrypto.pc
%exclude %{_cross_libdir}/crypto/cmake
%exclude %{_cross_libdir}/ssl/cmake

%files -n %{_cross_os}libssl-devel
%{_cross_pkgconfigdir}/libssl.pc
%{_cross_pkgconfigdir}/openssl.pc
