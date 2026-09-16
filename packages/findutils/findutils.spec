Name: %{_cross_os}findutils
Version: 4.11.0
Release: 1%{?dist}
Summary: A set of GNU tools for finding
License: GPL-3.0-or-later
URL: http://www.gnu.org/software/findutils/
Source0: https://ftp.gnu.org/pub/gnu/findutils/findutils-%{version}.tar.xz
Source1: https://ftp.gnu.org/pub/gnu/findutils/findutils-%{version}.tar.xz.sig
Source2: gpgkey-0CF4E8D871593224842832B888DD9E08C5DDACB9.asc
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libselinux-devel
Requires: %{_cross_os}libselinux

%description
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n findutils-%{version} -p1

%build
%cross_configure
%make_build

%install
%make_install

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_bindir}/find
%{_cross_bindir}/xargs
%exclude %{_cross_bindir}/locate
%exclude %{_cross_bindir}/updatedb
%exclude %{_cross_infodir}
%exclude %{_cross_libexecdir}
%exclude %{_cross_localedir}
%exclude %{_cross_mandir}

%changelog
