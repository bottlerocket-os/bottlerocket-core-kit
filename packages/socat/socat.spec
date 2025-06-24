Name: %{_cross_os}socat
Version: 1.8.0.3
Release: 1%{?dist}
Epoch: 1
Summary: Transfer data between two channels
License: GPL-2.0-only
URL: http://www.dest-unreach.org/socat/
Source0: http://www.dest-unreach.org/socat/download/socat-%{version}.tar.gz

BuildRequires: %{_cross_os}glibc-devel

%description
%{summary}.

%prep
%autosetup -n socat-%{version} -p0001

%build
%cross_configure \
  CFLAGS="-Wformat ${CFLAGS}" \
  --enable-help \
  --enable-ip4 \
  --enable-ip6 \
  --enable-listen \
  --enable-stdio \
  --enable-tcp \
  --enable-udp \
  --enable-unix \
  --disable-abstract-unixsocket \
  --disable-creat \
  --disable-dccp \
  --disable-exec \
  --disable-ext2 \
  --disable-fdnum \
  --disable-filan \
  --disable-file \
  --disable-fips \
  --disable-fs \
  --disable-genericsocket \
  --disable-gopen \
  --disable-interface \
  --disable-largefile \
  --disable-libwrap \
  --disable-namespaces \
  --disable-openssl \
  --disable-option-checking \
  --disable-pipe \
  --disable-posixmq \
  --disable-proxy \
  --disable-pty \
  --disable-rawip \
  --disable-readline \
  --disable-retry \
  --disable-sctp \
  --disable-shell \
  --disable-socketpair \
  --disable-socks4 \
  --disable-socks4a \
  --disable-socks5 \
  --disable-stats \
  --disable-sycls \
  --disable-system \
  --disable-termios \
  --disable-tun \
  --disable-udplite \
  --disable-vsock \

%make_build

%cross_generate_sbom

%install
%make_install

%cross_install_sbom

%files
%license COPYING
%{_cross_attribution_file}
%{_cross_sbom_package_dir}/%{name}-spdx.json
%{_cross_sbom_package_dir}/%{name}-cyclonedx.json
%{_cross_bindir}/socat
%{_cross_bindir}/socat1
%exclude %{_cross_bindir}/filan
%exclude %{_cross_bindir}/procan
%exclude %{_cross_bindir}/socat-broker.sh
%exclude %{_cross_bindir}/socat-chain.sh
%exclude %{_cross_bindir}/socat-mux.sh
%exclude %{_cross_mandir}/*

%changelog
