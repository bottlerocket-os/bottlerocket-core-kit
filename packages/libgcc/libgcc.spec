# This is a wrapper package that vends pre-built shared libraries from
# the SDK, allowing them to be loaded at runtime. It also lets us extract
# debuginfo in the usual way.
%undefine _debugsource_packages

Name: %{_cross_os}libgcc
Version: 0.0
Release: 1%{?dist}
Epoch: 1
Summary: GCC runtime library
License: GPL-3.0-or-later WITH GCC-exception-3.1
URL: https://gcc.gnu.org/

%description
%{summary}.

%package -n %{_cross_os}libstdc++
Summary: GCC C++ standard library
License: GPL-3.0-or-later WITH GCC-exception-3.1
Requires: %{_cross_os}libgcc

%description -n %{_cross_os}libstdc++
%{summary}.

%prep
%setup -T -c
cp %{_cross_licensedir}/gcc/COPYING{3,.RUNTIME} .

%build
install -p -m0755 %{_cross_libdir}/libgcc_s.so.1 .
install -p -m0755 %{_cross_libdir}/libstdc++.so.6.* .

%install
mkdir -p %{buildroot}%{_cross_libdir}
install -p -m0755 libgcc_s.so.1 %{buildroot}%{_cross_libdir}
install -p -m0755 libstdc++.so.6.* %{buildroot}%{_cross_libdir}
for lib in $(find %{buildroot}%{_cross_libdir} -name 'libstdc++.so.6.*') ; do
  ln -s "${lib##*/}" %{buildroot}%{_cross_libdir}/libstdc++.so.6
done

%files
%license COPYING3 COPYING.RUNTIME
%{_cross_attribution_file}
%{_cross_libdir}/libgcc_s.so.1

%files -n %{_cross_os}libstdc++
%{_cross_libdir}/libstdc++.so.6
%{_cross_libdir}/libstdc++.so.6.*

%changelog
