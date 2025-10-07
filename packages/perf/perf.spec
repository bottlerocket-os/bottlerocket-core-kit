Name: %{_cross_os}perf
Version: 6.18.10
Release: 1%{?dist}
Summary: The Linux kernel
License: GPL-2.0-only
URL: https://www.kernel.org/
Source0: https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-%{version}.tar.xz
Source1: https://cdn.kernel.org/pub/linux/kernel/v6.x/linux-%{version}.tar.sign
Source2: gpgkey-647F28654894E3BD457199BE38DBBDC86092693E.asc

BuildRequires: %{_cross_os}glibc-devel
BuildRequires: %{_cross_os}libelf-devel
Requires: %{_cross_os}libelf

%description
%{summary}.

%prep
%{gpgverify} --data=<(xzcat %{S:0}) --signature=%{S:1} --keyring=%{S:2}
%autosetup -n linux-%{version} -p1

%global pmake %{shrink: \
make -s \
  ARCH="%{_cross_karch}" \
  CROSS_COMPILE="%{_cross_target}-" \
  -C tools/perf \
  prefix="%{_cross_prefix}" \
  DESTDIR="%{buildroot}" \
  EXTRA_CFLAGS="%{_cross_cflags} -fno-lto" \
  EXTRA_CXXFLAGS="%{_cross_cxxflags} -fno-lto" \
  LDFLAGS="%{_cross_ldflags}" \
  NO_AIO=1 \
  NO_CAPSTONE=1 \
  NO_AUXTRACE=1 \
  NO_DEMANGLE=1 \
  NO_DWARF=1 \
  NO_GTK2=1 \
  NO_JEVENTS=1 \
  NO_JVMTI=1 \
  NO_LIBAUDIT=1 \
  NO_LIBBABELTRACE=1 \
  NO_LIBBIONIC=1 \
  NO_LIBBPF=1 \
  NO_LIBCAP=1 \
  NO_LIBCRYPTO=1 \
  NO_LIBDEBUGINFOD=1 \
  NO_LIBDW=1 \
  NO_LIBDW_DWARF_UNWIND=1 \
  NO_LIBLLVM=1 \
  NO_LIBNUMA=1 \
  NO_LIBPERL=1 \
  NO_LIBPFM4=1 \
  NO_LIBPYTHON=1 \
  NO_LIBTRACEEVENT=1 \
  NO_LIBUNWIND=1 \
  NO_LIBZSTD=1 \
  NO_LZMA=1 \
  NO_NEWT=1 \
  NO_PERF_READ_VDSO32=1 \
  NO_PERF_READ_VDSOX32=1 \
  NO_SDT=1 \
  NO_SHELLCHECK=1 \
  NO_SLANG=1 \
  NO_STRLCPY=1 \
  NO_ZLIB=1 \
  WERROR=0 \
  %{nil}}

%build
%pmake %{?_smp_mflags}

%install
%pmake %{?_smp_mflags} install-tools

%files
%license LICENSES/preferred/GPL-2.0
%{_cross_attribution_file}
%{_cross_bindir}/perf
%dir %{_cross_docdir}/perf-tip
%{_cross_docdir}/perf-tip/tips.txt
%exclude %{_cross_bindir}/trace
%exclude %{_cross_prefix}%{_sysconfdir}/bash_completion.d/perf
%exclude %{_cross_includedir}/perf/perf_dlfilter.h
%exclude %{_cross_libexecdir}/perf-core
