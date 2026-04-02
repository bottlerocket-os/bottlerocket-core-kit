%global _cross_first_party 1
%undefine _debugsource_packages
%global cross_generate_sbom %{shrink: \
  mkdir -p %{_builddir}/sbom-temp && \
  sbomtool generate \
    --name rottweiler \
    --out-dir %{_builddir}/sbom-temp \
    --build-dir %{_builddir}/sources \
    --spdx --cyclonedx}

# Skip the FIPS check, which has a false positive because of a symbol that
# starts with the "ring" prefix.
%undefine cross_check_fips

Name: %{_cross_os}rottweiler
Version: 0.1.0
Release: 1%{?dist}
Summary: Bottlerocket storage encryption helper
License: Apache-2.0 OR MIT
URL: https://github.com/bottlerocket-os/bottlerocket

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libbpf-devel
Requires: %{_cross_os}cryptsetup
Requires: %{_cross_os}libbpf
Requires: %{_cross_os}systemd-cryptsetup
Requires: %{_cross_os}tpm2-tools

%description
%{summary}.

%prep
%setup -T -c
%cargo_prep

%build
%cargo_build --manifest-path %{_builddir}/sources/Cargo.toml \
    -p rottweiler

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 %{__cargo_outdir}/rottweiler %{buildroot}%{_cross_bindir}
ln -s rottweiler %{buildroot}%{_cross_bindir}/rw

%files
%{_cross_bindir}/rottweiler
%{_cross_bindir}/rw
