Name: %{_cross_os}kexec-tools
Version: 2.0.32
Release: 1%{?dist}
Epoch: 1
Summary: Linux tool to load kernels from the running system
License: GPL-2.0-or-later AND GPL-2.0-only
URL: https://www.kernel.org/doc/html/latest/admin-guide/kdump/kdump.html
Source0: https://kernel.org/pub/linux/utils/kernel/kexec/kexec-tools-%{version}.tar.xz
Source1: https://kernel.org/pub/linux/utils/kernel/kexec/kexec-tools-%{version}.tar.sign
Source2: gpgkey-E27CD9A1F5ACC2FF4BFE7285D7CF64696A374FBE.asc

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libzstd-devel
Requires: %{_cross_os}libzstd

%description
%{summary}.

%prep
%{gpgverify} --data=<(xzcat %{S:0}) --signature=%{S:1} --keyring=%{S:2}
%setup -n kexec-tools-%{version}
rm -f kexec-tools.spec.in

%build
%cross_configure
%make_build

%install
%make_install

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_sbindir}/kexec
%exclude %{_cross_mandir}
%exclude %{_cross_sbindir}/vmcore-dmesg
%if "%{_cross_arch}" == "x86_64"
%exclude %{_cross_libdir}
%endif
