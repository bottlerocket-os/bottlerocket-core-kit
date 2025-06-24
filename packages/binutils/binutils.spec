Name: %{_cross_os}binutils
Version: 2.43.1
Release: 1%{?dist}
Epoch: 1
Summary: Tools for working with binaries
URL: https://sourceware.org/binutils
License: GPL-2.0-or-later AND LGPL-2.0-or-later AND GPL-3.0-or-later
Source0: https://ftp.gnu.org/gnu/binutils/binutils-%{version}.tar.xz
Source1: https://ftp.gnu.org/gnu/binutils/binutils-%{version}.tar.xz.sig
Source2: gpgkey-3A24BC1E8FB409FA9F14371813FCEF89DD9E3C4F.asc

Patch0001: 0001-PR32560-stack-buffer-overflow-at-objdump-disassemble.patch

Requires: %{_cross_os}libz
BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libz-devel

%description
%{summary}.

%package devel
Summary: Files for development using tools for working with binaries
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n binutils-%{version} -p1

%build
# Fail if the SDK version is different than the one provided in the image
[ %{version} = $(%{_cross_target}-ld -v | awk '{print $NF}') ] || exit 1

%cross_configure \
  --disable-gold \
  --disable-gdb \
  --with-system-zlib \
  --without-gnu-as \
  --disable-static \
  --disable-gprofng
%make_build MAKEINFO=true tooldir=%{_cross_prefix}
%cross_generate_sbom

%install
%make_install MAKEINFO=true tooldir=%{_cross_prefix}
%cross_install_sbom

%files
%license COPYING COPYING3.LIB COPYING3
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%{_cross_bindir}/ld
%{_cross_bindir}/strip
%dir %{_cross_libdir}/bfd-plugins
%{_cross_libdir}/bfd-plugins/libdep.so
%exclude %{_cross_infodir}
%exclude %{_cross_mandir}
%exclude %{_cross_localedir}
%exclude %{_cross_libdir}/ldscripts
%exclude %{_cross_bindir}/addr2line
%exclude %{_cross_bindir}/ar
%exclude %{_cross_bindir}/c++filt
%exclude %{_cross_bindir}/elfedit
%exclude %{_cross_bindir}/gprof
%exclude %{_cross_bindir}/ld.bfd
%exclude %{_cross_bindir}/nm
%exclude %{_cross_bindir}/objcopy
%exclude %{_cross_bindir}/objdump
%exclude %{_cross_bindir}/ranlib
%exclude %{_cross_bindir}/readelf
%exclude %{_cross_bindir}/size
%exclude %{_cross_bindir}/strings

%files devel
%{_cross_libdir}/*.a
%{_cross_includedir}/*.h
