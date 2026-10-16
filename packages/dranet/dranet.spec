%global goproject sigs.k8s.io
%global gorepo dranet
%global goimport %{goproject}/%{gorepo}

%global gover 1.5.0
%global rpmver %{gover}

%global _dwz_low_mem_die_limit 0

Name: %{_cross_os}dranet
Version: %{rpmver}
Release: 1%{?dist}
Summary: DRA driver for Kubernetes network devices
License: Apache-2.0
URL: https://github.com/kubernetes-sigs/dranet
Source0: https://github.com/kubernetes-sigs/%{gorepo}/archive/v%{gover}/%{gorepo}-%{gover}.tar.gz
Source1: bundled-%{gorepo}-%{gover}.tar.gz
Source2: dranet.service
Source3: dranet-exec-start-conf
Source4: dranet-disabled-marker
Source1000: clarify.toml

BuildRequires: git
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%prep
%autosetup -n %{gorepo}-%{gover} -p1
%setup -T -D -n %{gorepo}-%{gover} -b 1 -q

%build
export GO_MAJOR="1.26"

%set_cross_go_flags

go build -ldflags="${GOLDFLAGS}" -o dranet ./cmd/dranet

%install
install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 dranet %{buildroot}%{_cross_bindir}

install -d %{buildroot}%{_cross_unitdir}
install -p -m 0644 %{S:2} %{buildroot}%{_cross_unitdir}
install -d %{buildroot}%{_cross_unitdir}/dranet.service.d

install -D -m 0644 %{S:3} %{buildroot}%{_cross_templatedir}/dranet-exec-start-conf
install -D -m 0644 %{S:4} %{buildroot}%{_cross_templatedir}/dranet-disabled-marker

# The Go dependencies come from the bundled module archive, so scan them for
# attribution. The clarification file covers vendored licenses the scanner
# cannot detect on its own.
%cross_scan_attribution --clarify %{S:1000} go-vendor vendor

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%{_cross_bindir}/dranet
%{_cross_unitdir}/dranet.service
%dir %{_cross_unitdir}/dranet.service.d
%{_cross_templatedir}/dranet-exec-start-conf
%{_cross_templatedir}/dranet-disabled-marker

%changelog
