# Disable LTO since the performance critical code is all written in
# assembly, and optimizations applied to the C code could affect FIPS
# or overall correctness.
%global _cross_cflags %{_cross_cflags} -fno-lto
%global _cross_cxxflags %{_cross_cflags}

Name: %{_cross_os}libcrypto
Version: 3.0.0
Release: 1%{?dist}
Summary: AWS-LC cryptographic library
License: ISC AND (Apache-2.0 OR ISC) AND OpenSSL
URL: https://github.com/aws/aws-lc

Source0: https://github.com/aws/aws-lc/archive/AWS-LC-FIPS-%{version}/aws-lc-AWS-LC-FIPS-%{version}.tar.gz

# Upstream patches from AWS-LC-FIPS 3.0 branch can be fetched using the script
# at generate-aws-lc-patches.sh

Patch1001: 1001-Cherry-pick-BORINGSSL_bcm_text_hash-Go-utility-2221.patch
Patch1002: 1002-Cherry-pick-Fix-out-of-bound-OOB-input-read-in-AES-X.patch
Patch1003: 1003-Move-OCSP-ASN1-type-functions-to-public-header-2239.patch
Patch1004: 1004-Add-test-around-OpenSSL-behavior-for-BIO_get_mem_dat.patch
Patch1005: 1005-Cherry-pick-support-for-CMake-4.0-to-fips-2024-09-27.patch
Patch1006: 1006-Remove-some-indirection-in-SSL_certs_clear.patch
Patch1007: 1007-Add-SSL_CTX_use_cert_and_key-2163.patch
Patch1008: 1008-fips-2024-09-27-cherry-pick-FIPS-Integrity-Hash-Tool.patch
Patch1009: 1009-Adding-detection-of-out-of-bound-pre-bound-memory-re.patch
Patch1010: 1010-Avoid-mixing-SSE-and-AVX-in-XTS-mode-AVX512-implemen.patch
Patch1011: 1011-Update-BoringSSL-benchmark-to-use-C-17-2063.patch
Patch1012: 1012-FIPS-Cherry-pick-Support-allowing-specific-unknown-c.patch

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
%autosetup -n aws-lc-AWS-LC-FIPS-%{version} -p1

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

%cross_generate_sbom

%install
%ninja_install

%cross_install_sbom

%files
%license LICENSE NOTICE
%{_cross_attribution_file}
%{_cross_libdir}/libcrypto.so
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json

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
