Name: %{_cross_os}pciutils
Version: 3.13.0
Release: 1%{?dist}
Summary: PCI bus related utilities
License: GPL-2.0-only
URL: https://www.kernel.org/pub/software/utils/pciutils/
Source0:  https://mirrors.edge.kernel.org/pub/software/utils/pciutils/pciutils-%{version}.tar.gz
BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%prep
%autosetup -n pciutils-%{version} -p1

%global pciutils_make \
make\\\
  CROSS_COMPILE="%{_cross_target}-"\\\
  HOST="%{_cross_arch}-linux"\\\
  OPT="%{_cross_cflags}"\\\
  LDFLAGS="%{_cross_ldflags}"\\\
  PREFIX="%{_cross_prefix}"\\\
  LIBDIR="%{_cross_libdir}"\\\
  MANDIR="%{_cross_mandir}"\\\
  DESTDIR="%{buildroot}"\\\
  STRIP=""\\\
  SHARED=no\\\
  DNS=no\\\
  HWDB=no\\\
  LIBKMOD=no\\\
  ZLIB=no\\\
%{nil}

%build
%pciutils_make

%install
%pciutils_make install

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_bindir}/lspci
%{_cross_datadir}/pci.ids
%exclude %{_cross_sbindir}/pcilmr
%exclude %{_cross_sbindir}/setpci
%exclude %{_cross_sbindir}/update-pciids
%exclude %{_cross_mandir}
