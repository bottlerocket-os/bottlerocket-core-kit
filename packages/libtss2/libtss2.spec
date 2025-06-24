Name: %{_cross_os}libtss2
Version: 4.1.3
Release: 1%{?dist}
Summary: Libraries for the TPM 2.0 software stack
License: BSD-2-Clause
URL: https://github.com/tpm2-software/tpm2-tss
Source0: %{url}/releases/download/%{version}/tpm2-tss-%{version}.tar.gz
Source1: %{url}/releases/download/%{version}/tpm2-tss-%{version}.tar.gz.asc
Source2: gpgkey-D533275B0123D0A679F51FF48F4F9A45D7FFEE74.asc

Source10: tss-sysusers.conf

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libcrypto-devel

Requires: %{_cross_os}glibc
Requires: %{_cross_os}libcrypto

%description
%{summary}.

%package devel
Summary: Files for development using the libraries for the TPM 2.0 software stack
Requires: %{name}
Requires: %{_cross_os}libcrypto-devel

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n tpm2-tss-%{version}

%build
CONFIGURE_OPTS=(
  --disable-defaultflags
  --disable-doxygen-doc
  --disable-fapi
  --disable-integration
  --disable-log-file
  --disable-nodl
  --disable-policy
  --disable-silent-rules
  --disable-static
  --disable-tcti-cmd
  --disable-tcti-i2c-helper
  --disable-tcti-i2c-ftdi
  --disable-tcti-libtpms
  --disable-tcti-mssim
  --disable-tcti-pcap
  --disable-tcti-spi-ftdi
  --disable-tcti-spi-helper
  --disable-tcti-spi-ltt2go
  --disable-tcti-spidev
  --disable-tcti-swtpm
  --disable-unit
  --disable-weakcrypto
  --enable-esys
  --enable-tcti-device
  --with-crypto=ossl
  --with-maxloglevel=error
  --with-runstatedir=%{_rundir}
  --with-sysusersdir=%{_cross_sysusersdir}
  --with-tmpfilesdir=%{_cross_tmpfilesdir}
  --with-udevrulesdir=%{_cross_udevrulesdir}
)

%cross_configure "${CONFIGURE_OPTS[@]}"

%force_disable_rpath

%make_build

%cross_generate_sbom

%install
%make_install

install -d %{buildroot}%{_cross_sysusersdir}
install -p -m 0644 %{S:10} %{buildroot}%{_cross_sysusersdir}/tss.conf

%cross_install_sbom

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%{_cross_libdir}/libtss2-esys.so.*
%{_cross_libdir}/libtss2-rc.so.*
%{_cross_libdir}/libtss2-mu.so.*
%{_cross_libdir}/libtss2-sys.so.*
%{_cross_libdir}/libtss2-tctildr.so.*
%{_cross_libdir}/libtss2-tcti-device.so.*
%{_cross_sysusersdir}/tss.conf
%{_cross_udevrulesdir}/tpm-udev.rules
%exclude %{_cross_mandir}

%files devel
%{_cross_libdir}/libtss2-esys.so
%{_cross_libdir}/libtss2-rc.so
%{_cross_libdir}/libtss2-mu.so
%{_cross_libdir}/libtss2-sys.so
%{_cross_libdir}/libtss2-tctildr.so
%{_cross_libdir}/libtss2-tcti-device.so
%{_cross_includedir}/tss2
%{_cross_pkgconfigdir}/tss2-esys.pc
%{_cross_pkgconfigdir}/tss2-rc.pc
%{_cross_pkgconfigdir}/tss2-mu.pc
%{_cross_pkgconfigdir}/tss2-sys.pc
%{_cross_pkgconfigdir}/tss2-tctildr.pc
%{_cross_pkgconfigdir}/tss2-tcti-device.pc
