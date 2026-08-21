%global goproject github.com/aws
%global gorepo amazon-ssm-agent
%global goimport %{goproject}/%{gorepo}

Name: %{_cross_os}amazon-ssm-agent
Version: 3.3.5068.0
Release: 1%{?dist}
Summary: An agent to enable remote management of EC2 instances
License: Apache-2.0
URL: https://github.com/aws/amazon-ssm-agent
Source0: %{gorepo}-%{version}.tar.gz
Source1: amazon-ssm-agent.service
Source2: amazon-ssm-agent.json
Source1000: clarify.toml

Patch0001: 0001-agent-Add-config-to-make-shell-optional.patch

BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%package plugin
Summary: A statically-linked agent to enable remote management of EC2 instances

%description plugin
%{summary}.

%prep
%autosetup -n %{gorepo}-%{version} -p1

%build
%set_cross_go_flags

export GOTOOLCHAIN=local

go build -ldflags "${GOLDFLAGS}" -o amazon-ssm-agent \
  ./core/agent.go ./core/agent_unix.go ./core/agent_parser.go

go build -ldflags "${GOLDFLAGS}" -o ssm-agent-worker \
  ./agent/agent.go ./agent/agent_unix.go ./agent/agent_parser.go

go build -ldflags "${GOLDFLAGS}" -o ssm-session-worker \
  ./agent/framework/processor/executer/outofproc/sessionworker/main.go

%set_cross_go_flags_static

go build -ldflags "${GOLDFLAGS}" -o static/amazon-ssm-agent \
  ./core/agent.go ./core/agent_unix.go ./core/agent_parser.go

go build -ldflags "${GOLDFLAGS}" -o static/ssm-agent-worker \
  ./agent/agent.go ./agent/agent_unix.go ./agent/agent_parser.go

go build -ldflags "${GOLDFLAGS}" -o static/ssm-session-worker \
  ./agent/framework/processor/executer/outofproc/sessionworker/main.go

%install
install -D -p -m 0644 %{S:1} %{buildroot}%{_cross_unitdir}/amazon-ssm-agent.service

install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/amazon/ssm
install -m 0644 %{S:2} %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/amazon/ssm/amazon-ssm-agent.json

install -d %{buildroot}%{_cross_bindir}
for b in amazon-ssm-agent ssm-agent-worker ssm-session-worker; do
  install -p -m 0755 ${b} %{buildroot}%{_cross_bindir}
done

# Install the statically-linked SSM agent under 'libexecdir', since it is meant to be used by other programs
install -d %{buildroot}%{_cross_libexecdir}/amazon-ssm-agent/bin/%{version}
for b in amazon-ssm-agent ssm-agent-worker ssm-session-worker; do
  install -p -m 0755 static/${b} %{buildroot}%{_cross_libexecdir}/amazon-ssm-agent/bin/%{version}
done

%cross_scan_attribution --clarify %{S:1000} go-vendor vendor

ln -sf %{version} %{buildroot}%{_cross_libexecdir}/amazon-ssm-agent/bin/latest

%files
%license LICENSE
%{_cross_attribution_file}
%{_cross_attribution_vendor_dir}
%{_cross_unitdir}/amazon-ssm-agent.service
%dir %{_cross_factorydir}%{_cross_sysconfdir}/amazon/ssm
%{_cross_factorydir}%{_cross_sysconfdir}/amazon/ssm/amazon-ssm-agent.json
%{_cross_bindir}/amazon-ssm-agent
%{_cross_bindir}/ssm-agent-worker
%{_cross_bindir}/ssm-session-worker

%files plugin
%dir %{_cross_libexecdir}/amazon-ssm-agent
%{_cross_libexecdir}/amazon-ssm-agent/bin/%{version}/amazon-ssm-agent
%{_cross_libexecdir}/amazon-ssm-agent/bin/%{version}/ssm-agent-worker
%{_cross_libexecdir}/amazon-ssm-agent/bin/%{version}/ssm-session-worker
%{_cross_libexecdir}/amazon-ssm-agent/bin/latest
