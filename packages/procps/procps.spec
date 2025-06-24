Name: %{_cross_os}procps
Version: 4.0.5
Release: 1%{?dist}
Summary: A set of process monitoring tools
License: GPL-2.0-or-later AND LGPL-2.1-or-later
URL: https://gitlab.com/procps-ng/procps
Source0: https://gitlab.com/procps-ng/procps/-/archive/v%{version}/procps-v%{version}.tar.gz

# Upstream patch to fix warning about format truncation.
Patch0001: 0001-library-internal-expand-buffer-for-stat_fd.patch

# Local patch to fix warning about implicit definition of pidfd_open.
Patch1001: 1001-check-for-sys-pidfd.h.patch

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libselinux-devel
Requires: %{_cross_os}libselinux

%description
%{summary}.

%package devel
Summary: Files for development using the process monitoring tools
Requires: %{name}

%description devel
%{summary}.

%prep
%autosetup -n procps-v%{version} -p1

%build
./autogen.sh
%cross_configure \
  --enable-libselinux \
  --enable-skill \
  --disable-kill \
  --disable-modern-top \
  --disable-nls \
  --disable-w-from \
  --without-ncurses \
  --without-systemd \

%force_disable_rpath

%make_build

%cross_generate_sbom

%install
%make_install

# Replace identical binaries with symlinks to avoid duplicate build ID warnings.
declare -A aliases=(
  [pgrep]="pkill"
  [pidwait]="pkill"
  [snice]="skill"
)

pushd %{buildroot}/%{_cross_bindir}
for a in ${!aliases[*]} ; do
  b="${aliases[${a}]}"
  if cmp --quiet ${b} ${a} ; then
    ln -snf "${b}" "${a}"
  fi
done
popd

%cross_install_sbom

%files
%license COPYING COPYING.LIB
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%{_cross_bindir}/free
%{_cross_bindir}/pgrep
%{_cross_bindir}/pidof
%{_cross_bindir}/pkill
%{_cross_bindir}/pmap
%{_cross_bindir}/ps
%{_cross_bindir}/pwdx
%{_cross_bindir}/pidwait
%{_cross_bindir}/skill
%{_cross_bindir}/snice
%{_cross_bindir}/tload
%{_cross_bindir}/uptime
%{_cross_bindir}/vmstat
%{_cross_bindir}/w
%{_cross_sbindir}/sysctl
%{_cross_libdir}/*.so.*

%exclude %{_cross_docdir}/*
%exclude %{_cross_mandir}/*

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.so
%dir %{_cross_includedir}/libproc2
%{_cross_includedir}/libproc2/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
