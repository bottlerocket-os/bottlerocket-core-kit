# Disable LTO since the performance critical code is all written in
# assembly, and optimizations applied to the C code could affect FIPS
# or overall correctness.
%global _cross_cflags %{_cross_cflags} -fno-lto
%global _cross_cxxflags %{_cross_cflags}

Name: %{_cross_os}libcrypto
Version: 3.1.0
Release: 1%{?dist}
Summary: AWS-LC cryptographic library
License: ISC AND (Apache-2.0 OR ISC) AND OpenSSL
URL: https://github.com/aws/aws-lc

Source0: https://github.com/aws/aws-lc/archive/AWS-LC-FIPS-%{version}/aws-lc-AWS-LC-FIPS-%{version}.tar.gz

# Upstream patches from AWS-LC-FIPS 3.1 branch can be fetched using the script
# at generate-aws-lc-patches.sh

Patch1019: 1019-Cherry-pick-2024-Offer-P521-for-signature_algorithms.patch
Patch1020: 1020-1-byte-OOB-read-in-EVP_PKEY_asn1_find_str-length-cal.patch
Patch1021: 1021-pkcs8-cap-ciphertext-length-before-allocating-in-pkc.patch
Patch1022: 1022-evp-disable-EVP_PKEY_derive-for-KEM-method.patch
Patch1023: 1023-reject-zero-sized-digests-in-HKDF-EVP_PKEY.patch
Patch1024: 1024-Reject-XOF-digests-in-DH_compute_key_hashed.patch
Patch1025: 1025-Prepare-v3.2.0-3050.patch
Patch1026: 1026-Use-CRYPTO_memcmp-instead-of-OPENSSL_memcmp-for-tag-.patch
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
