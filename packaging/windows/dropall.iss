; SPDX-License-Identifier: GPL-3.0-or-later
; Installer di Dropall per Windows (Inno Setup 6).
; Compilazione (fatta in automatico da GitHub):
;   ISCC.exe /DAppVersion=0.3.0 /DSourceExe=C:\percorso\dropall-windows-x86_64.exe dropall.iss
; Installa solo per l'utente corrente (nessun permesso di amministratore): così l'app
; può aggiornarsi da sola, perché la sua cartella è scrivibile.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\..\dropall-windows-x86_64.exe"
#endif

[Setup]
; identificativo fisso: serve a riconoscere e aggiornare un'installazione esistente
AppId={{AE84E872-3ECF-4C11-A7E3-3284AD074F26}
AppName=Dropall
AppVersion={#AppVersion}
AppVerName=Dropall {#AppVersion}
AppPublisher=Dropall
DefaultDirName={autopf}\Dropall
DefaultGroupName=Dropall
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
OutputBaseFilename=Dropall-Setup-windows-x86_64
SetupIconFile=..\..\assets\icon.ico
UninstallDisplayIcon={app}\Dropall.exe
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
Name: "torrentassoc"; Description: "Open magnet links and .torrent files with Dropall / Apri link magnet e file .torrent con Dropall"; GroupDescription: "Torrent:"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "Dropall.exe"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\..\THIRD_PARTY_LICENSES.txt"; DestDir: "{app}"; Flags: ignoreversion

[Registry]
; solo per l'utente corrente (HKCU): nessun permesso di amministratore
Root: HKCU; Subkey: "Software\Classes\magnet"; ValueType: string; ValueName: ""; ValueData: "URL:Magnet link"; Flags: uninsdeletekey; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\magnet"; ValueType: string; ValueName: "URL Protocol"; ValueData: ""; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\magnet\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\Dropall.exe,0"; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\magnet\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\Dropall.exe"" ""%1"""; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\.torrent"; ValueType: string; ValueName: ""; ValueData: "Dropall.torrent"; Flags: uninsdeletevalue; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\Dropall.torrent"; ValueType: string; ValueName: ""; ValueData: "Torrent"; Flags: uninsdeletekey; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\Dropall.torrent\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\Dropall.exe,0"; Tasks: torrentassoc
Root: HKCU; Subkey: "Software\Classes\Dropall.torrent\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\Dropall.exe"" ""%1"""; Tasks: torrentassoc

[Icons]
Name: "{group}\Dropall"; Filename: "{app}\Dropall.exe"
Name: "{autodesktop}\Dropall"; Filename: "{app}\Dropall.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Dropall.exe"; Description: "{cm:LaunchProgram,Dropall}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; l'app può aggiornarsi da sola e lasciare file temporanei nascosti accanto all'eseguibile.
; Si tolgono solo quelli: la cartella sparisce da sola se resta vuota (l'utente potrebbe
; averla scelta lui e contenere altri file).
Type: files; Name: "{app}\.*.exe"
