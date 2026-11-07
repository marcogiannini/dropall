; SPDX-License-Identifier: GPL-3.0-or-later
; Installer di Grabbit per Windows (Inno Setup 6).
; Compilazione (fatta in automatico da GitHub):
;   ISCC.exe /DAppVersion=0.3.0 /DSourceExe=C:\percorso\grabbit-windows-x86_64.exe grabbit.iss
; Installa solo per l'utente corrente (nessun permesso di amministratore): così l'app
; può aggiornarsi da sola, perché la sua cartella è scrivibile.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\..\grabbit-windows-x86_64.exe"
#endif

[Setup]
; identificativo fisso: serve a riconoscere e aggiornare un'installazione esistente
AppId={{6C1F0D5B-8E2A-4B7C-A3D9-5F4E2B9C7A10}
AppName=Grabbit
AppVersion={#AppVersion}
AppVerName=Grabbit {#AppVersion}
AppPublisher=Grabbit
DefaultDirName={autopf}\Grabbit
DefaultGroupName=Grabbit
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputBaseFilename=Grabbit-Setup-windows-x86_64
SetupIconFile=..\..\assets\icon.ico
UninstallDisplayIcon={app}\Grabbit.exe
LicenseFile=..\..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
; lingua del sistema senza chiedere: italiano se il PC è in italiano, altrimenti inglese
ShowLanguageDialog=auto

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "torrentassoc"; Description: "Open magnet links and .torrent files with Grabbit / Apri link magnet e file .torrent con Grabbit"; GroupDescription: "Torrent:"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "Grabbit.exe"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\..\THIRD_PARTY_LICENSES.txt"; DestDir: "{app}"; Flags: ignoreversion

[Registry]
; solo per l'utente corrente (HKCU): nessun permesso di amministratore
Root: HKCU; Subkey: "Software\Classes\magnet"; ValueType: string; ValueName: ""; ValueData: "URL:Magnet link"; Flags: uninsdeletekey; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\magnet"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\magnet\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\Grabbit.exe,0"; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\magnet\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\Grabbit.exe"" ""%1"""; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\.torrent"; ValueType: string; ValueName: ""; ValueData: "Grabbit.torrent"; Flags: uninsdeletevalue; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\Grabbit.torrent"; ValueType: string; ValueName: ""; ValueData: "Torrent"; Flags: uninsdeletekey; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\Grabbit.torrent\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\Grabbit.exe,0"; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\Grabbit.torrent\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\Grabbit.exe"" ""%1"""; Tasks: torrentassoc

[Icons]
Name: "{group}\Grabbit"; Filename: "{app}\Grabbit.exe"
Name: "{autodesktop}\Grabbit"; Filename: "{app}\Grabbit.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Grabbit.exe"; Description: "{cm:LaunchProgram,Grabbit}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; l'app può aggiornarsi da sola e lasciare file temporanei nascosti accanto all'eseguibile.
; Si tolgono solo quelli: la cartella sparisce da sola se resta vuota (l'utente potrebbe
; averla scelta lui e contenere altri file).
Type: files; Name: "{app}\.*.exe"
