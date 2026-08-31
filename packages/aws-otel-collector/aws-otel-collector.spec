%global goproject github.com/aws-observability
%global gorepo aws-otel-collector
%global goimport %{goproject}/%{gorepo}

Name: %{_cross_os}aws-otel-collector
Version: 0.49.0
Release: 1%{?dist}
Epoch: 1
Summary: AWS Distro for OpenTelemetry Collector
License: Apache-2.0 AND BSD-2-Clause AND BSD-3-Clause AND MIT AND MPL-2.0
URL: https://github.com/aws-observability/aws-otel-collector
Source0: %{gorepo}-v%{version}.tar.gz
Source1: aws-otel-collector.service
Source2: aws-otel-collector-tmpfiles.conf
Source3: aws-otel-collector.yaml

# Change log and extraconfig file paths from /opt to /var/log and /etc, respectively
Patch0001: 0001-change-logger-and-extraconfig-file-paths.patch

BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%prep
%autosetup -n %{gorepo}-%{version} -p1

%build

%set_cross_go_flags
export GO_MAJOR="1.26"
go build -ldflags "${GOLDFLAGS}" -o aws-otel-collector ./cmd/awscollector

%install
install -D -p -m 0644 %{S:1} %{buildroot}%{_cross_unitdir}/aws-otel-collector.service

install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m 0644 %{S:2} %{buildroot}%{_cross_tmpfilesdir}

install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}
install -p -m 0644 %{S:3} %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}

install -d %{buildroot}%{_cross_bindir}
install -p -m 0755 aws-otel-collector %{buildroot}%{_cross_bindir}

%files
%{_cross_attribution_file}
%{_cross_bindir}/aws-otel-collector
%{_cross_unitdir}/aws-otel-collector.service
%{_cross_tmpfilesdir}/aws-otel-collector-tmpfiles.conf
%{_cross_factorydir}%{_cross_sysconfdir}/aws-otel-collector.yaml

