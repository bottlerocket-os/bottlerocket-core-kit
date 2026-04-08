Name: %{_cross_os}bash
Version: 5.3
Release: 1%{?dist}
Summary: The GNU Bourne Again shell
License: GPL-3.0-or-later
URL: https://www.gnu.org/software/bash
Source0: https://ftp.gnu.org/gnu/bash/bash-%{version}.tar.gz
Source1: https://ftp.gnu.org/gnu/bash/bash-%{version}.tar.gz.sig
Source2: gpgkey-7C0135FB088AAF6C66C650B9BB5869F064EA74AB.asc

# Disable loadable builtin examples
Patch127: bash-4.4-no-loadable-builtins.patch

Patch1001: bash53-001
Patch1002: bash53-002
Patch1003: bash53-003
Patch1004: bash53-004
Patch1005: bash53-005
Patch1006: bash53-006
Patch1007: bash53-007
Patch1008: bash53-008
Patch1009: bash53-009

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libncurses-devel
BuildRequires: %{_cross_os}readline-devel
Requires: %{_cross_os}libncurses
Requires: %{_cross_os}readline

# Provide `/bin/sh` at a lower priority than `brush`, since this package
# should only be installed if specifically requested.
Provides: %{_cross_os}package-file(/bin/sh) = 0:
Conflicts: %{_cross_os}brush

%description
%{summary}.

%package devel
Summary: Files for development using the GNU Bourne Again shell
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -n bash-%{version} -N
%autopatch -p1 -m0001 -M1000
%autopatch -p0 -m1001

echo %{version} > _distribution
echo %{release} > _patchlevel

# force refreshing the generated files
rm y.tab.*

%build
(
export \
  ac_cv_rl_prefix="%{_cross_sysroot}" \
  ac_cv_rl_version="8.0" \
  bash_cv_decl_under_sys_siglist=yes \
  bash_cv_dup2_broken=no \
  bash_cv_func_ctype_nonascii=yes \
  bash_cv_func_sbrk=yes \
  bash_cv_func_sigsetjmp=present \
  bash_cv_func_strcoll_broken=no \
  bash_cv_func_snprintf=yes \
  bash_cv_func_vsnprintf=yes \
  bash_cv_fnmatch_equiv_fallback=yes \
  bash_cv_getcwd_malloc=yes \
  bash_cv_getenv_redef=yes \
  bash_cv_job_control_missing=present \
  bash_cv_must_reinstall_sighandlers=no \
  bash_cv_opendir_not_robust=no \
  bash_cv_pgrp_pipe=yes \
  bash_cv_printf_a_format=yes \
  bash_cv_sys_named_pipes=present \
  bash_cv_sys_siglist=yes \
  bash_cv_ulimit_maxfds=yes \
  bash_cv_under_sys_siglist=yes \
  bash_cv_unusable_rtsigs=no \
  bash_cv_wcontinued_broken=no \
  bash_cv_wexitstatus_offset=8 ;
%cross_configure \
  --with-bash-malloc=no \
  --with-installed-readline
)

make "CPPFLAGS=-D_GNU_SOURCE -DRECYCLES_PIDS -DDEFAULT_PATH_VALUE='\"/usr/local/bin:/usr/bin\"'" %{?_smp_mflags}

%install
%make_install install-headers
ln -s bash %{buildroot}%{_cross_bindir}/sh

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_bindir}/bash
%{_cross_bindir}/sh
%exclude %{_cross_bindir}/bashbug
%exclude %{_cross_datadir}/doc/*
%exclude %{_cross_datadir}/locale/*
%exclude %{_cross_infodir}/*
%exclude %{_cross_mandir}/*

%files devel
%dir %{_cross_includedir}/bash
%dir %{_cross_includedir}/bash/builtins
%dir %{_cross_includedir}/bash/include
%{_cross_includedir}/bash/*.h
%{_cross_includedir}/bash/builtins/*.h
%{_cross_includedir}/bash/include/*.h
%{_cross_pkgconfigdir}/*.pc

%changelog
