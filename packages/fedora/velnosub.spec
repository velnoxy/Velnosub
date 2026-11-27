Name:           velnosub
Version:        0.1.1
Release:        1%{?dist}
Summary:        Advanced ASS and SRT subtitle editor
License:        BSD-3-Clause AND ISC AND MIT AND MPL-1.1
URL:            https://github.com/velnoxy/Velnosub
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  gcc-c++
BuildRequires:  git-core
BuildRequires:  meson
BuildRequires:  ninja-build
BuildRequires:  gettext
BuildRequires:  appstream
BuildRequires:  desktop-file-utils
BuildRequires:  boost-devel
BuildRequires:  gtest-devel
BuildRequires:  gmock-devel
BuildRequires:  wxGTK-devel
BuildRequires:  pkgconfig(alsa)
BuildRequires:  pkgconfig(ffms2)
BuildRequires:  pkgconfig(fftw3)
BuildRequires:  pkgconfig(fontconfig)
BuildRequires:  pkgconfig(gl)
BuildRequires:  pkgconfig(hunspell)
BuildRequires:  pkgconfig(icu-i18n)
BuildRequires:  pkgconfig(icu-uc)
BuildRequires:  pkgconfig(libass)
BuildRequires:  pkgconfig(libpulse)
BuildRequires:  pkgconfig(luajit)
BuildRequires:  pkgconfig(openal)
BuildRequires:  pkgconfig(uchardet)
BuildRequires:  pkgconfig(zlib)
Requires:       hicolor-icon-theme

%description
Velnosub is a subtitle editor based on Aegisub, with audio timing,
video preview, ASS typesetting and Lua automation.

%prep
%autosetup
mkdir -p %{_vpath_builddir}
cp git_version.h %{_vpath_builddir}/git_version.h

%build
%meson --datadir=share/velnosub --localedir=share/velnosub/locale -Dffms2=enabled -Davisynth=disabled -Dbestsource=disabled -Dvapoursynth=disabled
%meson_build

%install
%meson_install
mv %{buildroot}%{_bindir}/aegisub %{buildroot}%{_bindir}/velnosub
mkdir -p %{buildroot}%{_datadir}/{applications,icons,metainfo}
desktop-file-install --set-key=Name --set-value=Velnosub \
    --set-key=Exec --set-value='velnosub %f' --set-icon=velnosub \
    --dir=%{buildroot}%{_datadir}/applications \
    %{buildroot}%{_datadir}/velnosub/applications/aegisub.desktop
mv %{buildroot}%{_datadir}/applications/{aegisub,com.velnoxy.velnosub}.desktop
mv %{buildroot}%{_datadir}/velnosub/icons/hicolor %{buildroot}%{_datadir}/icons/
find %{buildroot}%{_datadir}/icons/hicolor -type f -name 'aegisub.*' -exec sh -c \
    'for path do mv "$path" "${path%/*}/velnosub.${path##*.}"; done' sh {} +
sed -e 's|<id>aegisub.desktop</id>|<id>com.velnoxy.velnosub</id>|' \
    -e 's|>aegisub.desktop</launchable>|>com.velnoxy.velnosub.desktop</launchable>|' \
    -e 's|>aegisub</binary>|>velnosub</binary>|' \
    -e 's|<name>Aegisub</name>|<name>Velnosub</name>|' \
    %{buildroot}%{_datadir}/velnosub/metainfo/com.velnoxy.velnosub.appdata.xml \
    > %{buildroot}%{_datadir}/metainfo/com.velnoxy.velnosub.appdata.xml
rm %{buildroot}%{_datadir}/velnosub/{applications/aegisub.desktop,metainfo/com.velnoxy.velnosub.appdata.xml}
desktop-file-validate %{buildroot}%{_datadir}/applications/com.velnoxy.velnosub.desktop

%check
%meson_test

%files
%license LICENCE
%doc README.md
%{_bindir}/velnosub
%{_datadir}/applications/com.velnoxy.velnosub.desktop
%{_datadir}/icons/hicolor/*/apps/velnosub.*
%{_datadir}/metainfo/com.velnoxy.velnosub.appdata.xml
%{_datadir}/velnosub/
