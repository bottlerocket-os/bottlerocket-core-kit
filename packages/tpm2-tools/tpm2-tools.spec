Name: %{_cross_os}tpm2-tools
Version: 5.7
Release: 1%{?dist}
Summary: Tools for the TPM 2.0 software stack
License: BSD-3-Clause
URL: https://github.com/tpm2-software/tpm2-tools
Source0: %{url}/releases/download/%{version}/tpm2-tools-%{version}.tar.gz
Source1: %{url}/releases/download/%{version}/tpm2-tools-%{version}.tar.gz.asc
Source2: gpgkey-D533275B0123D0A679F51FF48F4F9A45D7FFEE74.asc

# aws-lc doesn't have SM2 or SM3
Patch0001: 0001-tpm2-tools-disable-SM2-and-SM3-checks.patch

# libcurl isn't available
Patch0002: 0002-tpm2-tools-disable-tpm2_getekcertificate.patch

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libtss2-devel

Requires: %{_cross_os}glibc
Requires: %{_cross_os}libtss2

%description
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n tpm2-tools-%{version} -p1

%build
autoreconf -fi

CONFIGURE_OPTS=(
  --disable-fapi
  --disable-silent-rules
  --disable-static
)

%cross_configure "${CONFIGURE_OPTS[@]}"

%make_build
%cross_generate_sbom

%install
%make_install
%cross_install_sbom

%files
%license docs/LICENSE
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%{_cross_bindir}/tpm2
%{_cross_bindir}/tpm2_activatecredential
%{_cross_bindir}/tpm2_certify
%{_cross_bindir}/tpm2_certifyX509certutil
%{_cross_bindir}/tpm2_certifycreation
%{_cross_bindir}/tpm2_changeauth
%{_cross_bindir}/tpm2_changeeps
%{_cross_bindir}/tpm2_changepps
%{_cross_bindir}/tpm2_checkquote
%{_cross_bindir}/tpm2_clear
%{_cross_bindir}/tpm2_clearcontrol
%{_cross_bindir}/tpm2_clockrateadjust
%{_cross_bindir}/tpm2_commit
%{_cross_bindir}/tpm2_create
%{_cross_bindir}/tpm2_createak
%{_cross_bindir}/tpm2_createek
%{_cross_bindir}/tpm2_createpolicy
%{_cross_bindir}/tpm2_createprimary
%{_cross_bindir}/tpm2_dictionarylockout
%{_cross_bindir}/tpm2_duplicate
%{_cross_bindir}/tpm2_ecdhkeygen
%{_cross_bindir}/tpm2_ecdhzgen
%{_cross_bindir}/tpm2_ecephemeral
%{_cross_bindir}/tpm2_encodeobject
%{_cross_bindir}/tpm2_encryptdecrypt
%{_cross_bindir}/tpm2_eventlog
%{_cross_bindir}/tpm2_evictcontrol
%{_cross_bindir}/tpm2_flushcontext
%{_cross_bindir}/tpm2_getcap
%{_cross_bindir}/tpm2_getcommandauditdigest
%{_cross_bindir}/tpm2_geteccparameters
%{_cross_bindir}/tpm2_getpolicydigest
%{_cross_bindir}/tpm2_getrandom
%{_cross_bindir}/tpm2_getsessionauditdigest
%{_cross_bindir}/tpm2_gettestresult
%{_cross_bindir}/tpm2_gettime
%{_cross_bindir}/tpm2_hash
%{_cross_bindir}/tpm2_hierarchycontrol
%{_cross_bindir}/tpm2_hmac
%{_cross_bindir}/tpm2_import
%{_cross_bindir}/tpm2_incrementalselftest
%{_cross_bindir}/tpm2_load
%{_cross_bindir}/tpm2_loadexternal
%{_cross_bindir}/tpm2_makecredential
%{_cross_bindir}/tpm2_nvcertify
%{_cross_bindir}/tpm2_nvdefine
%{_cross_bindir}/tpm2_nvextend
%{_cross_bindir}/tpm2_nvincrement
%{_cross_bindir}/tpm2_nvread
%{_cross_bindir}/tpm2_nvreadlock
%{_cross_bindir}/tpm2_nvreadpublic
%{_cross_bindir}/tpm2_nvsetbits
%{_cross_bindir}/tpm2_nvundefine
%{_cross_bindir}/tpm2_nvwrite
%{_cross_bindir}/tpm2_nvwritelock
%{_cross_bindir}/tpm2_pcrallocate
%{_cross_bindir}/tpm2_pcrevent
%{_cross_bindir}/tpm2_pcrextend
%{_cross_bindir}/tpm2_pcrread
%{_cross_bindir}/tpm2_pcrreset
%{_cross_bindir}/tpm2_policyauthorize
%{_cross_bindir}/tpm2_policyauthorizenv
%{_cross_bindir}/tpm2_policyauthvalue
%{_cross_bindir}/tpm2_policycommandcode
%{_cross_bindir}/tpm2_policycountertimer
%{_cross_bindir}/tpm2_policycphash
%{_cross_bindir}/tpm2_policyduplicationselect
%{_cross_bindir}/tpm2_policylocality
%{_cross_bindir}/tpm2_policynamehash
%{_cross_bindir}/tpm2_policynv
%{_cross_bindir}/tpm2_policynvwritten
%{_cross_bindir}/tpm2_policyor
%{_cross_bindir}/tpm2_policypassword
%{_cross_bindir}/tpm2_policypcr
%{_cross_bindir}/tpm2_policyrestart
%{_cross_bindir}/tpm2_policysecret
%{_cross_bindir}/tpm2_policysigned
%{_cross_bindir}/tpm2_policytemplate
%{_cross_bindir}/tpm2_policyticket
%{_cross_bindir}/tpm2_print
%{_cross_bindir}/tpm2_quote
%{_cross_bindir}/tpm2_rc_decode
%{_cross_bindir}/tpm2_readclock
%{_cross_bindir}/tpm2_readpublic
%{_cross_bindir}/tpm2_rsadecrypt
%{_cross_bindir}/tpm2_rsaencrypt
%{_cross_bindir}/tpm2_selftest
%{_cross_bindir}/tpm2_send
%{_cross_bindir}/tpm2_sessionconfig
%{_cross_bindir}/tpm2_setclock
%{_cross_bindir}/tpm2_setcommandauditstatus
%{_cross_bindir}/tpm2_setprimarypolicy
%{_cross_bindir}/tpm2_shutdown
%{_cross_bindir}/tpm2_sign
%{_cross_bindir}/tpm2_startauthsession
%{_cross_bindir}/tpm2_startup
%{_cross_bindir}/tpm2_stirrandom
%{_cross_bindir}/tpm2_testparms
%{_cross_bindir}/tpm2_tr_encode
%{_cross_bindir}/tpm2_unseal
%{_cross_bindir}/tpm2_verifysignature
%{_cross_bindir}/tpm2_zgen2phase
%exclude %{_cross_bashdir}
%exclude %{_cross_mandir}
