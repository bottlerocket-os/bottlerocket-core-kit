%global goproject github.com/kubernetes-sigs
%global gorepo aws-iam-authenticator
%global goimport %{goproject}/%{gorepo}

%global gover 0.7.10
%global rpmver %{gover}

%global _dwz_low_mem_die_limit 0

Name: %{_cross_os}%{gorepo}
Version: %{rpmver}
Release: 1%{?dist}
Epoch: 1
Summary: AWS IAM authenticator
License: Apache-2.0
URL: https://%{goimport}
Source0: https://%{goimport}/archive/v%{gover}/%{gorepo}-%{gover}.tar.gz
Source1: bundled-aws-iam-authenticator-%{gover}.tar.gz
Source1000: clarify.toml

BuildRequires: git
BuildRequires: %{_cross_os}glibc-devel

# For IAM Roles Anywhere, the signing helper might be set as the credential
# process.
Requires: %{_cross_os}aws-signing-helper

%description
%{summary}.

%prep
%setup -n %{gorepo}-%{gover} -q
%setup -T -D -n %{gorepo}-%{version} -b 1

%build
%set_cross_go_flags
export GO_MAJOR="1.25"
go build -ldflags="${GOLDFLAGS}" -o aws-iam-authenticator ./cmd/aws-iam-authenticator

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 aws-iam-authenticator %{buildroot}%{_cross_bindir}

%cross_scan_attribution --clarify %{S:1000} go-vendor vendor

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%{_cross_bindir}/aws-iam-authenticator
%changelog
