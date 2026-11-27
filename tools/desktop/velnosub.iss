#ifndef Payload
  #error Payload directory required
#endif
#ifndef Output
  #error Output directory required
#endif
#ifndef DesktopVersion
  #error Desktop version required
#endif

[Setup]
AppName=Velnosub
AppVersion={#DesktopVersion}
AppPublisher=Velnosub
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
DisableProgramGroupPage=yes
WizardStyle=modern
OutputDir={#Output}
Compression=lzma2
SolidCompression=yes
#ifdef Portable
AppId=VelnosubPortable
DefaultDirName={userdocs}\Velnosub Portable
OutputBaseFilename=Velnosub-windows-x64-portable
Uninstallable=no
CreateUninstallRegKey=no
UsePreviousAppDir=no
UsePreviousLanguage=no
#else
AppId=Velnosub
DefaultDirName={localappdata}\Programs\Velnosub
DefaultGroupName=Velnosub
OutputBaseFilename=Velnosub-windows-x64-setup
UninstallDisplayIcon={app}\Velnosub.exe
#endif

[Files]
Source: "{#Payload}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

#ifndef Portable
[Icons]
Name: "{group}\Velnosub"; Filename: "{app}\Velnosub.exe"
#endif

[Run]
Filename: "{app}\Velnosub.exe"; Description: "Start Velnosub"; Flags: nowait postinstall skipifsilent
