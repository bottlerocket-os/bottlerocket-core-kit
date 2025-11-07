%global _cross_first_party 1
%global workspace_name notation-image-verifier

Name: %{_cross_os}%{workspace_name}
Version: 0.1.0
Release: 1%{?dist}
Summary: A notation-based containerd image verification plugin
License: Apache-2.0 OR MIT
URL: https://github.com/bottlerocket-os/bottlerocket
BuildRequires: %{_cross_os}glibc-devel
Requires: %{_cross_os}notation
Requires: %{_cross_os}aws-signer-notation-plugin
Requires: %{name}(binaries)

Source1: containerd-image-verifiers-toml

%description
%{summary}.

%package bin
Summary: A notation-based containerd image verification plugin binaries
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(no-fips) and %{name})
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-fips-bin)

%description bin
%{summary}.

%package fips-bin
Summary: A notation-based containerd image verification plugin binaries, FIPS edition
Provides: %{name}(binaries)
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-bin)

%description fips-bin
%{summary}.

%prep
%setup -T -c
cp -r %{_builddir}/sources/%{workspace_name}/* .

%build
%set_cross_go_flags
go build -ldflags="${GOLDFLAGS}" -o notation-image-verifier .
gofips build -ldflags="${GOLDFLAGS}" -o fips/notation-image-verifier .

%install
install -d %{buildroot}%{_cross_libexecdir}/image-verifiers/bin
install -p -m 0755 notation-image-verifier %{buildroot}%{_cross_libexecdir}/image-verifiers/bin

install -d %{buildroot}%{_cross_fips_libexecdir}/image-verifiers/bin
install -p -m 0755 fips/notation-image-verifier %{buildroot}%{_cross_fips_libexecdir}/image-verifiers/bin

install -d %{buildroot}%{_cross_templatedir}
install -p -m 0644 %{S:1} %{buildroot}%{_cross_templatedir}

%files
%dir %{_cross_libexecdir}/image-verifiers/bin
%dir %{_cross_fips_libexecdir}/image-verifiers/bin
%{_cross_templatedir}/containerd-image-verifiers-toml

%files bin
%{_cross_libexecdir}/image-verifiers/bin/notation-image-verifier

%files fips-bin
%{_cross_fips_libexecdir}/image-verifiers/bin/notation-image-verifier

%changelog
