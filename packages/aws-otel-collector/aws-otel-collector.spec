%global goproject github.com/aws-observability
%global gorepo aws-otel-collector
%global goimport %{goproject}/%{gorepo}

Name: %{_cross_os}aws-otel-collector
Version: 0.41.1
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
Requires: %{name}(binary)

%description
%{summary}.

%package bin
Summary: Telemetry collector binary
Provides: %{name}(binary)
Requires: (%{_cross_os}image-feature(no-fips) and %{name})
Conflicts: (%{_cross_os}image-feature(fips) or %{name}-fips-bin)

%description bin
%{summary}.

%package fips-bin
Summary: Telemetry collector binary, FIPS edition
Provides: %{name}(binary)
Requires: (%{_cross_os}image-feature(fips) and %{name})
Conflicts: (%{_cross_os}image-feature(no-fips) or %{name}-bin)

%description fips-bin
%{summary}.

%prep
%autosetup -n %{gorepo}-%{version} -p1

%build
export GO_MAJOR="1.22"

%set_cross_go_flags

go build -ldflags "${GOLDFLAGS}" -o aws-otel-collector ./cmd/awscollector
gofips build -ldflags "${GOLDFLAGS}" -o fips/aws-otel-collector ./cmd/awscollector

%install
install -D -p -m 0644 %{S:1} %{buildroot}%{_cross_unitdir}/aws-otel-collector.service

install -d %{buildroot}%{_cross_tmpfilesdir}
install -p -m 0644 %{S:2} %{buildroot}%{_cross_tmpfilesdir}

install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}
install -p -m 0644 %{S:3} %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}

install -d %{buildroot}{%{_cross_bindir},%{_cross_fips_bindir}}
install -p -m 0755 aws-otel-collector %{buildroot}%{_cross_bindir}
install -p -m 0755 fips/aws-otel-collector %{buildroot}%{_cross_fips_bindir}

%files
%{_cross_attribution_file}
%{_cross_unitdir}/aws-otel-collector.service
%{_cross_tmpfilesdir}/aws-otel-collector-tmpfiles.conf
%{_cross_factorydir}%{_cross_sysconfdir}/aws-otel-collector.yaml

%files bin
%{_cross_bindir}/aws-otel-collector

%files fips-bin
%{_cross_fips_bindir}/aws-otel-collector
