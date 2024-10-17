%global _cross_first_party 1
%global early_boot_config_bindir %{_cross_libexecdir}/early-boot-config/bin
%global early_boot_config_fips_bindir %{_cross_fips_libexecdir}/early-boot-config/bin
%global early_boot_config_provider_dir %{_cross_libexecdir}/early-boot-config/data-providers.d
%undefine _debugsource_packages

Name: %{_cross_os}early-boot-config
Version: 0.1
Release: 1%{?dist}
Epoch: 1
Summary: early-boot-config
License: Apache-2.0 OR MIT
URL: https://github.com/bottlerocket-os/bottlerocket

Source100: early-boot-config.service

BuildRequires: %{_cross_os}glibc-devel

Requires: (%{name}-aws if %{_cross_os}variant-platform(aws))
Requires: (%{name}-vmware if %{_cross_os}variant-platform(vmware))
Requires: (%{name}-metal if %{_cross_os}variant-platform(metal))

%description
%{summary}.

%package local
Summary: local-provider

%description local
%{summary}.

%package aws
Summary: early-boot-config package for AWS
Requires: %{name}
Requires: %{name}-local
Requires: %{name}-aws(binaries)

%description aws
%{summary}.

%package aws-bin
Summary: early-boot-config binaries for AWS
Provides: %{name}-aws(binaries)
Requires: (%{_cross_os}image-feature(no-fips) and %{name}-aws)
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-aws-fips-bin)

%description aws-bin
%{summary}.

%package aws-fips-bin
Summary: early-boot-config binaries for AWS, FIPS edition
Provides: %{name}-aws(binaries)
Requires: (%{_cross_os}image-feature(fips) and %{name}-aws)
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-aws-bin)

%description aws-fips-bin
%{summary}.

%ifarch x86_64
%package vmware
Summary: early-boot-config package for vmware
Requires: %{name}
Requires: %{name}-local

%description vmware
%{summary}.
%endif

%package metal
Summary: early-boot-config package for metal
Requires: %{name}
Requires: %{name}-local

%description metal
%{summary}.

%prep
%setup -T -c
%cargo_prep

# Some of the AWS-LC sources are built with `-O0`. This is not compatible with
# `-Wp,-D_FORTIFY_SOURCE=2`, which needs at least `-O2`.
sed -i 's/-Wp,-D_FORTIFY_SOURCE=2//g' \
  %_cross_cmake_toolchain_conf \
  %_cross_cmake_toolchain_conf_static

%build
%cargo_build --manifest-path %{_builddir}/sources/Cargo.toml \
    -p early-boot-config \
    -p ec2-identity-doc-user-data-provider \
    -p ec2-imds-user-data-provider \
    -p local-defaults-user-data-provider \
    -p local-file-user-data-provider \
    -p local-overrides-user-data-provider \
%ifarch x86_64
    -p vmware-cd-rom-user-data-provider \
    -p vmware-guestinfo-user-data-provider \
%endif
    %{nil}

# Store the output so we can print it after waiting for the backgrounded job.
exec 3>&1 4>&2
fips_output="$(mktemp)"
exec 1>"${fips_output}" 2>&1
# Build FIPS binaries in the background
%cargo_build_fips --manifest-path %{_builddir}/sources/Cargo.toml \
    -p ec2-identity-doc-user-data-provider \
    -p ec2-imds-user-data-provider \
    &
# Save the PID so we can wait for it later.
fips_pid="$!"
exec 1>&3 2>&4

# Wait for fips builds from the background, if they're not already done.
set +e; wait "${fips_pid}"; fips_rc="${?}"; set -e
echo -e "\n** Output from FIPS builds:"
cat "${fips_output}"

if [ "${fips_rc}" -ne 0 ]; then
   exit "${fips_rc}"
fi

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 %{__cargo_outdir}/early-boot-config %{buildroot}%{_cross_bindir}

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:100} %{buildroot}%{_cross_unitdir}

install -d %{buildroot}%{early_boot_config_bindir}
install -d %{buildroot}%{early_boot_config_fips_bindir}

for p in ec2-identity-doc-user-data-provider ec2-imds-user-data-provider; do
  install -p -m 0755 %{__cargo_outdir}/${p} %{buildroot}%{early_boot_config_bindir}
  install -p -m 0755 %{__cargo_outdir_fips}/${p} %{buildroot}%{early_boot_config_fips_bindir}
done

install -p -m 0755 \
    %{__cargo_outdir}/ec2-identity-doc-user-data-provider \
    %{__cargo_outdir}/ec2-imds-user-data-provider \
    %{__cargo_outdir}/local-defaults-user-data-provider \
    %{__cargo_outdir}/local-file-user-data-provider \
    %{__cargo_outdir}/local-overrides-user-data-provider \
    %{buildroot}%{early_boot_config_bindir}

%ifarch x86_64
install -p -m 0755 \
    %{__cargo_outdir}/vmware-cd-rom-user-data-provider \
    %{__cargo_outdir}/vmware-guestinfo-user-data-provider \
    %{buildroot}%{early_boot_config_bindir}
%endif

install -d %{buildroot}%{early_boot_config_provider_dir}

ln -rs \
  %{buildroot}%{early_boot_config_bindir}/ec2-identity-doc-user-data-provider \
  %{buildroot}%{early_boot_config_provider_dir}/30-ec2-identity-doc

ln -rs \
  %{buildroot}%{early_boot_config_bindir}/ec2-imds-user-data-provider \
  %{buildroot}%{early_boot_config_provider_dir}/40-ec2-imds

ln -rs \
  %{buildroot}%{early_boot_config_bindir}/local-defaults-user-data-provider \
  %{buildroot}%{early_boot_config_provider_dir}/10-local-defaults

ln -rs \
  %{buildroot}%{early_boot_config_bindir}/local-file-user-data-provider \
  %{buildroot}%{early_boot_config_provider_dir}/20-local-user-data

ln -rs \
  %{buildroot}%{early_boot_config_bindir}/local-overrides-user-data-provider \
  %{buildroot}%{early_boot_config_provider_dir}/99-local-overrides

%ifarch x86_64
ln -rs \
  %{buildroot}%{early_boot_config_bindir}/vmware-cd-rom-user-data-provider \
  %{buildroot}%{early_boot_config_provider_dir}/30-vmware-cd-rom

ln -rs \
  %{buildroot}%{early_boot_config_bindir}/vmware-guestinfo-user-data-provider \
  %{buildroot}%{early_boot_config_provider_dir}/40-vmware-guestinfo
%endif

%files
%{_cross_bindir}/early-boot-config
%{_cross_unitdir}/early-boot-config.service
%dir %{early_boot_config_provider_dir}

%files local
%{early_boot_config_bindir}/local-defaults-user-data-provider
%{early_boot_config_bindir}/local-file-user-data-provider
%{early_boot_config_bindir}/local-overrides-user-data-provider
%{early_boot_config_provider_dir}/10-local-defaults
%{early_boot_config_provider_dir}/20-local-user-data
%{early_boot_config_provider_dir}/99-local-overrides

%files aws
%{early_boot_config_provider_dir}/30-ec2-identity-doc
%{early_boot_config_provider_dir}/40-ec2-imds

%files aws-bin
%{early_boot_config_bindir}/ec2-identity-doc-user-data-provider
%{early_boot_config_bindir}/ec2-imds-user-data-provider

%files aws-fips-bin
%{early_boot_config_fips_bindir}/ec2-identity-doc-user-data-provider
%{early_boot_config_fips_bindir}/ec2-imds-user-data-provider

%ifarch x86_64
%files vmware
%{early_boot_config_bindir}/vmware-cd-rom-user-data-provider
%{early_boot_config_bindir}/vmware-guestinfo-user-data-provider
%{early_boot_config_provider_dir}/30-vmware-cd-rom
%{early_boot_config_provider_dir}/40-vmware-guestinfo
%endif

# There are no metal-specific providers, just dependencies like the local file providers.
%files metal
