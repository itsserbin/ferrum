#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

#define AppName "Ferrum"
#define AppExeName "ferrum.exe"
#define AppPublisher "itsserbin"
#define AppURL "https://github.com/itsserbin/ferrum"

[Setup]
AppId={{B4E7C8A1-5D3F-4E2A-9C6B-8F1D0E7A3B5C}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppURL}
AppSupportURL={#AppURL}/issues
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputBaseFilename=Ferrum-Setup-x64
SetupIconFile=..\..\assets\icon.ico
UninstallDisplayIcon={app}\{#AppExeName}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesEnvironment=yes
MinVersion=10.0

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "startmenu"; Description: "Create Start Menu shortcut"; GroupDescription: "Shortcuts:";
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "addtopath"; Description: "Add to PATH"; GroupDescription: "System integration:";

[Files]
Source: "ferrum.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: startmenu
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Code]
procedure EnvAddPath(Path: string; IsSystem: Boolean);
var
  Paths: string;
  RootKey: Integer;
begin
  if IsSystem then
    RootKey := HKLM
  else
    RootKey := HKCU;

  if IsSystem then
    RegQueryStringValue(RootKey, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', Paths)
  else
    RegQueryStringValue(RootKey, 'Environment', 'Path', Paths);

  if Pos(';' + Uppercase(Path) + ';', ';' + Uppercase(Paths) + ';') > 0 then
    exit;

  if Paths = '' then
    Paths := Path
  else
    Paths := Paths + ';' + Path;

  if IsSystem then
    RegWriteExpandStringValue(RootKey, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', Paths)
  else
    RegWriteExpandStringValue(RootKey, 'Environment', 'Path', Paths);
end;

procedure EnvRemovePath(Path: string; IsSystem: Boolean);
var
  Paths: string;
  P: Integer;
  RootKey: Integer;
begin
  if IsSystem then
    RootKey := HKLM
  else
    RootKey := HKCU;

  if IsSystem then begin
    if not RegQueryStringValue(RootKey, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', Paths) then
      exit;
  end else begin
    if not RegQueryStringValue(RootKey, 'Environment', 'Path', Paths) then
      exit;
  end;

  P := Pos(';' + Uppercase(Path), Uppercase(Paths));
  if P = 0 then begin
    P := Pos(Uppercase(Path) + ';', Uppercase(Paths));
    if P = 0 then begin
      if Uppercase(Paths) = Uppercase(Path) then
        Paths := ''
      else
        exit;
    end else
      Delete(Paths, P, Length(Path) + 1);
  end else
    Delete(Paths, P, Length(Path) + 1);

  if IsSystem then
    RegWriteExpandStringValue(RootKey, 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment', 'Path', Paths)
  else
    RegWriteExpandStringValue(RootKey, 'Environment', 'Path', Paths);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and IsTaskSelected('addtopath') then
    EnvAddPath(ExpandConstant('{app}'), IsAdminInstallMode());
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then begin
    EnvRemovePath(ExpandConstant('{app}'), True);
    EnvRemovePath(ExpandConstant('{app}'), False);
  end;
end;
