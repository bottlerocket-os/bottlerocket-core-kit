Name: %{_cross_os}glibc
Version: 2.44
Release: 1%{?dist}
Epoch: 1
Summary: The GNU libc libraries
License: LGPL-2.1-or-later AND (LGPL-2.1-or-later WITH GCC-exception-2.0) AND GPL-2.0-or-later AND (GPL-2.0-or-later WITH GCC-exception-2.0) AND BSD-3-Clause AND ISC
URL: http://www.gnu.org/software/glibc/
Source0: https://ftp.gnu.org/gnu/glibc/glibc-%{version}.tar.xz
Source1: https://ftp.gnu.org/gnu/glibc/glibc-%{version}.tar.xz.sig
Source2: gpgkey-35B17DF5752577CA0C541CEB94BFDF4484AD142F.asc

Source11: glibc-tmpfiles.conf
Source12: ld.so.conf
Source13: ldconfig-service.conf
Source14: tz-utc.txt

# We include this patch as a source file to have more control over how it's
# applied and reverted during the build.
Source99: HACK-only-build-and-install-localedef.patch

# Upstream patches from 2.44 release branch:
# ```
# git checkout origin/release/2.44/master
# git format-patch --no-numbered --no-signature glibc-2.44..
# ```
Patch0001: 0001-advisories-replace-with-ADVISORIES-text-file.patch
Patch0002: 0002-NEWS-start-2.44.1-section.patch
Patch0003: 0003-hurd-Make-the-readlink-__fstatat64-references-option.patch
Patch0004: 0004-hurd-fix-fork-s-longjmp-demangling-on-i386.patch
Patch0005: 0005-math-Fix-sinh-worst-case-results-for-x-36.736801-BZ-.patch
Patch0006: 0006-math-Fix-x86_64-tanh-_FloatN-aliases-binding-to-the-.patch
Patch0007: 0007-io-fix-ftw-ABI-on-MIPS-n64.patch
Patch0008: 0008-hurd-Fix-build-after-the-ftw-kernel_stat.h-inclusion.patch
Patch0009: 0009-linux-Inline-syscall-cancellation-to-keep-wrapper-fr.patch
Patch0010: 0010-Fix-gen-as-const-headers-races-with-the-parallel-sub.patch
Patch0011: 0011-Makerules-Only-install-the-ABI-lib-names-header-from.patch
Patch0012: 0012-Makefile-Only-print-the-test-summary-in-the-second-p.patch
Patch0013: 0013-Makerules-Make-the-.dt-to-.d-conversion-safe-against.patch
Patch0014: 0014-Makefile-Order-the-top-level-stamp-files-before-the-.patch
Patch0015: 0015-arm-Order-the-rtld-link-after-libgcc-stubs.a.patch
Patch0016: 0016-benchtests-Create-objdir-in-the-bench-.c-generation-.patch
Patch0017: 0017-elf-test-handle-different-rootsbindir-in-tst-ldconfi.patch
Patch0018: 0018-ldbl-opt-Fix-mlong-double-128-configure-test-for-Cla.patch
Patch0019: 0019-powerpc-Fix-mlong-double-128-IBM-format-configure-te.patch
Patch0020: 0020-stdio-common-run-AWK-in-the-C-locale-in-the-printf-f.patch
Patch0021: 0021-stdio-common-avoid-repeated-regexp-matches-in-tst-pr.patch
Patch0022: 0022-string-Speed-up-strcmp-test-data-initialization.patch
Patch0023: 0023-string-Speed-up-strcasecmp-test-data-initialization.patch
Patch0024: 0024-elf-Honour-skip_ifunc-for-cross-object-IFUNC-relocat.patch
Patch0025: 0025-m68k-Fix-fmod-fmodf-infinite-recursion-BZ-34508.patch
Patch0026: 0026-misc-Fix-out-of-bounds-array-write-in-tdelete-bug-34.patch
Patch0027: 0027-m68k-remove-sysdeps-m68k-m680x0-fpu-w_fmod_compat.c-.patch
Patch0028: 0028-posix-Remove-unnecessary-overflow-check-in-wordexp-B.patch
Patch0029: 0029-stdlib-Fix-right-justification-in-strfmon-bug-34510-.patch
Patch0030: 0030-iconvdata-SHIFT_JISX0213-decoding-lacks-pending-char.patch
Patch0031: 0031-iconvdata-EUC_JISX0213-decoding-lacks-pending-charac.patch
Patch0032: 0032-iconvdata-Test-case-for-bug-34556-bug-34568.patch
Patch0033: 0033-alpha-Fix-stack-alignment-in-makecontext.patch
Patch0034: 0034-alpha-add-the-denormal-trap-enable-bit-to-FE_NOMASK_.patch
Patch0035: 0035-alpha-expect-test-float32x-float64-div-to-fail.patch
Patch0036: 0036-Add-check-symbol-version.awk.patch
Patch0037: 0037-powerpc-Fix-non-atomic-stack-pointer-update-in-forti.patch
Patch0038: 0038-powerpc-Fix-preprocessor-conditional-in-soft-float-_.patch
Patch0039: 0039-libio-Fix-CVE-2026-18374-heap-buffer-overflow-in-ccs.patch
Patch0040: 0040-libio-Add-test-for-fopen-with-an-empty-ccs-value-BZ-.patch
Patch0041: 0041-nptl-Skip-pretty-printer-tests-without-python3-BZ-34.patch
Patch0042: 0042-elf-Do-not-load-cache-extensions-from-an-old-format-.patch
Patch0043: 0043-io-drop-nonnull-attribute-for-fchmodat-faccessat-fch.patch
Patch0044: 0044-x86-64-Link-tst-shstk-legacy-1-f-g-with-Wl-no-as-nee.patch
Patch0045: 0045-Revert-io-drop-nonnull-attribute-for-fchmodat-facces.patch
Patch0046: 0046-io-drop-nonnull-attribute-for-fchmodat-faccessat-fch.patch
Patch0047: 0047-fcntl-drop-nonnull-attribute-for-openat-openat2-s-pa.patch
Patch0048: 0048-resolv-Fix-assertion-failure-on-search-list-truncati.patch

# Fedora patches
Patch1001: glibc-cs-path.patch

# Local patches
Patch9001: 9001-move-ldconfig-cache-to-ephemeral-storage.patch
Patch9002: 9002-Revert-malloc-auto-enable-THP-on-aarch64.patch

%description
%{summary}.

%package devel
Summary: Files for development using the GNU libc libraries.
Requires: %{name}

%description devel
%{summary}.

%prep
%{gpgverify} --data=%{S:0} --signature=%{S:1} --keyring=%{S:2}
%autosetup -Sgit -n glibc-%{version} -p1

%global glibc_configure %{shrink: \
BUILDFLAGS="-O2 -g1 -Wp,-D_GLIBCXX_ASSERTIONS -fstack-clash-protection -fno-omit-frame-pointer -mno-omit-leaf-frame-pointer" \
CFLAGS="${BUILDFLAGS}" CPPFLAGS="" CXXFLAGS="${BUILDFLAGS}" \
../configure \
  --prefix="%{_cross_prefix}" \
  --sysconfdir="%{_cross_sysconfdir}" \
  --localstatedir="%{_cross_localstatedir}" \
  --enable-bind-now \
  --enable-fortify-source \
  --enable-multi-arch \
  --enable-shared \
  --enable-stack-protector=strong \
  --disable-build-nscd \
  --disable-crypt \
  --disable-nscd \
  --disable-profile \
  --disable-systemtap \
  --disable-timezone-tools \
  --without-gd \
  --without-selinux
  %{nil}}

%build

# First build the host tools we need, namely `localedef`. Apply a patch from
# Buildroot that allows us to build just this program and not everything.
patch -p1 < %{S:99}

mkdir build
pushd build
%glibc_configure
make %{?_smp_mflags} -O -r locale/others
mv locale/localedef %{_builddir}/localedef
popd

# Remove the previous build, revert the patch, and verify that the tree is
# clean, since we don't want to contaminate our target build.
rm -rf build
patch -p1 -R < %{S:99}
git diff --quiet

# Now build for the target. This is what will end up in the package, except
# for the C.UTF-8 locale, which we need `localedef` to generate.
mkdir build
pushd build
CC="%{_cross_target}-gcc %{?_cross_arch_cflags}" CXX="%{_cross_target}-g++ %{?_cross_arch_cflags}" \
%glibc_configure \
  --target="%{_cross_target}" \
  --host="%{_cross_target}" \
  --build="%{_build}" \
  --with-headers="%{_cross_includedir}" \
  --enable-kernel="5.10.0"
make %{?_smp_mflags} -O -r
popd

%install
pushd build
make -j1 install_root=%{buildroot} install
# By default, LOCALEDEF refers to the target binary, and is invoked by the
# dynamic linker that was just built for the target. Neither will run on a
# build host with a different architecture. The locale format is compatible
# across architectures but not across glibc versions, so we can't rely on
# the binary in the SDK and must use the one we built earlier.
make -j1 install_root=%{buildroot} install-files-C.UTF-8/UTF-8 -C ../localedata objdir="$(pwd)" \
  LOCALEDEF="I18NPATH=. GCONV_PATH=$(pwd)/../iconvdata LC_ALL=C %{_builddir}/localedef"
popd

install -d %{buildroot}%{_cross_tmpfilesdir}
install -d %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}
install -d %{buildroot}%{_cross_unitdir}/ldconfig.service.d

install -p -m 0644 %{S:11} %{buildroot}%{_cross_tmpfilesdir}/glibc.conf
install -p -m 0644 %{S:12} %{buildroot}%{_cross_factorydir}%{_cross_sysconfdir}/ld.so.conf
install -p -m 0644 %{S:13} %{buildroot}%{_cross_unitdir}/ldconfig.service.d/ldconfig.conf

truncate -s 0 %{buildroot}%{_cross_libdir}/gconv/gconv-modules
chmod 644 %{buildroot}%{_cross_libdir}/gconv/gconv-modules
truncate -s 0 %{buildroot}%{_cross_libdir}/gconv/gconv-modules.cache
chmod 644 %{buildroot}%{_cross_libdir}/gconv/gconv-modules.cache

truncate -s 0 %{buildroot}%{_cross_datadir}/locale/locale.alias
chmod 644 %{buildroot}%{_cross_datadir}/locale/locale.alias

install -d %{buildroot}%{_cross_datadir}/zoneinfo
base64 --decode %{S:14} > %{buildroot}%{_cross_datadir}/zoneinfo/UTC

%files
%license COPYING.LIB COPYINGv2 COPYINGv3 LICENSES
%{_cross_attribution_file}
%{_cross_tmpfilesdir}/glibc.conf
%exclude %{_cross_sysconfdir}/rpc

%{_cross_bindir}/getconf
%{_cross_bindir}/getent
%exclude %{_cross_bindir}/gencat
%exclude %{_cross_bindir}/iconv
%exclude %{_cross_bindir}/ld.so
%exclude %{_cross_bindir}/ldd
%exclude %{_cross_bindir}/locale
%exclude %{_cross_bindir}/localedef
%exclude %{_cross_bindir}/makedb
%exclude %{_cross_bindir}/mtrace
%exclude %{_cross_bindir}/pldd
%exclude %{_cross_bindir}/pcprofiledump
%exclude %{_cross_bindir}/sotruss
%exclude %{_cross_bindir}/sprof
%exclude %{_cross_bindir}/xtrace

%{_cross_sbindir}/ldconfig
%exclude %{_cross_sbindir}/iconvconfig
%exclude %{_cross_sbindir}/sln

%dir %{_cross_libexecdir}/getconf
%{_cross_libexecdir}/getconf/*

%{_cross_libdir}/ld-linux-*.so.*
%{_cross_libdir}/libBrokenLocale.so.*
%{_cross_libdir}/libanl.so.*
%{_cross_libdir}/libc.so.*
%{_cross_libdir}/libdl.so.*
%{_cross_libdir}/libm.so.*
%{_cross_libdir}/libnss_dns.so.*
%{_cross_libdir}/libnss_files.so.*
%{_cross_libdir}/libpthread.so.*
%{_cross_libdir}/libresolv.so.*
%{_cross_libdir}/librt.so.*
%{_cross_libdir}/libthread_db.so.*
%{_cross_libdir}/libutil.so.*
%{_cross_libdir}/libmvec.so.*
%exclude %{_cross_libdir}/audit/sotruss-lib.so
%exclude %{_cross_libdir}/libc_malloc_debug.so.*
%exclude %{_cross_libdir}/libmemusage.so
%exclude %{_cross_libdir}/libpcprofile.so
%exclude %{_cross_libdir}/libnsl.so.*
%exclude %{_cross_libdir}/libnss_compat.so.*
%exclude %{_cross_libdir}/libnss_db.so.*
%exclude %{_cross_libdir}/libnss_hesiod.so.*

%dir %{_cross_libdir}/gconv
%dir %{_cross_libdir}/gconv/gconv-modules.d
%{_cross_libdir}/gconv/gconv-modules
%{_cross_libdir}/gconv/gconv-modules.cache
%exclude %{_cross_libdir}/gconv/*.so
%exclude %{_cross_libdir}/gconv/gconv-modules.d/*.conf

%dir %{_cross_libdir}/locale
%dir %{_cross_libdir}/locale/C.utf8
%{_cross_libdir}/locale/C.utf8/LC_*

%dir %{_cross_datadir}/i18n
%dir %{_cross_datadir}/i18n/charmaps
%dir %{_cross_datadir}/i18n/locales
%dir %{_cross_datadir}/locale
%{_cross_datadir}/locale/locale.alias
%dir %{_cross_datadir}/zoneinfo
%{_cross_datadir}/zoneinfo/UTC
%exclude %{_cross_datadir}/i18n/charmaps/*
%exclude %{_cross_datadir}/i18n/locales/*
%exclude %{_cross_datadir}/locale/*
%exclude %{_cross_localstatedir}/db/Makefile

%dir %{_cross_factorydir}
%{_cross_factorydir}%{_cross_sysconfdir}/ld.so.conf

%dir %{_cross_unitdir}/ldconfig.service.d
%{_cross_libdir}/systemd/system/ldconfig.service.d/ldconfig.conf

%files devel
%{_cross_libdir}/*.a
%{_cross_libdir}/*.o
%{_cross_libdir}/libBrokenLocale.so
%{_cross_libdir}/libanl.so
%{_cross_libdir}/libc.so
%{_cross_libdir}/libm.so
%{_cross_libdir}/libresolv.so
%{_cross_libdir}/libthread_db.so
%{_cross_libdir}/libmvec.so
%exclude %{_cross_libdir}/libc_malloc_debug.so
%exclude %{_cross_libdir}/libnss_compat.so
%exclude %{_cross_libdir}/libnss_db.so
%exclude %{_cross_libdir}/libnss_hesiod.so

%dir %{_cross_includedir}/arpa
%dir %{_cross_includedir}/bits
%dir %{_cross_includedir}/gnu
%dir %{_cross_includedir}/net
%dir %{_cross_includedir}/netinet
%dir %{_cross_includedir}/netipx
%dir %{_cross_includedir}/netiucv
%dir %{_cross_includedir}/netpacket
%dir %{_cross_includedir}/netrose
%dir %{_cross_includedir}/nfs
%dir %{_cross_includedir}/protocols
%dir %{_cross_includedir}/rpc
%dir %{_cross_includedir}/scsi
%dir %{_cross_includedir}/sys
%dir %{_cross_includedir}/netash
%dir %{_cross_includedir}/netatalk
%dir %{_cross_includedir}/netax25
%dir %{_cross_includedir}/neteconet
%dir %{_cross_includedir}/netrom
%{_cross_includedir}/*.h
%{_cross_includedir}/*/*

%changelog
