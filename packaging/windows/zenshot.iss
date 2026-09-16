#ifndef MyAppVersion
  #define MyAppVersion "2.6.0"
#endif

#define MyAppName "ZenShot"
#define MyAppExeName "zenshot.exe"
#define RepoRoot AddBackslash(SourcePath) + "..\.."

[Setup]
AppId={{7E3C9A1B-4D2F-4C8A-9B11-ZENSHOT2026}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} {#MyAppVersion}
AppPublisher=ZenShot
AppPublisherURL=https://github.com/masudranaxpert/zenshot
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile={#RepoRoot}\LICENSE
OutputDir={#RepoRoot}
OutputBaseFilename=zenshot-setup-{#MyAppVersion}
SetupIconFile={#RepoRoot}\assets\zenshot.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
RestartApplications=no
MinVersion=10.0
ChangesAssociations=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "autostart"; Description: "Start {#MyAppName} with Windows"; GroupDescription: "Additional options:"; Flags: checkedonce
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional options:"; Flags: unchecked

[Files]
Source: "{#RepoRoot}\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RepoRoot}\README.md"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "{#RepoRoot}\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Comment: "ZenShot tray"
Name: "{autoprograms}\{#MyAppName}\Take Screenshot"; Filename: "{app}\{#MyAppExeName}"; Parameters: "--capture"
Name: "{autoprograms}\{#MyAppName}\Options"; Filename: "{app}\{#MyAppExeName}"; Parameters: "--options"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "ZenShot"; ValueData: """{app}\{#MyAppExeName}"""; Flags: uninsdeletevalue; Tasks: autostart

[Run]
Filename: "{app}\{#MyAppExeName}"; Parameters: "--enable-autostart"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent; Tasks: autostart
Filename: "{app}\{#MyAppExeName}"; Parameters: "--disable-autostart"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent; Tasks: not autostart

[UninstallRun]
Filename: "{cmd}"; Parameters: "/C taskkill /IM {#MyAppExeName} /F"; Flags: runhidden; RunOnceId: "StopZenShot"
