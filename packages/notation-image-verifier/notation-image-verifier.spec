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

Source1: containerd-image-verifiers-toml

%description
%{summary}.

%prep
%setup -T -c
cp -r %{_builddir}/sources/%{workspace_name}/* .

%build
%set_cross_go_flags
go build -ldflags="${GOLDFLAGS}" -o notation-image-verifier .

%install
install -d %{buildroot}%{_cross_libexecdir}/image-verifiers/bin
install -p -m 0755 notation-image-verifier %{buildroot}%{_cross_libexecdir}/image-verifiers/bin

install -d %{buildroot}%{_cross_templatedir}
install -p -m 0644 %{S:1} %{buildroot}%{_cross_templatedir}

%files
%dir %{_cross_libexecdir}/image-verifiers/bin
%{_cross_templatedir}/containerd-image-verifiers-toml
%{_cross_libexecdir}/image-verifiers/bin/notation-image-verifier

%changelog
