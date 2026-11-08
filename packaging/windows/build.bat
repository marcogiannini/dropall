@echo off
REM Compila Dropall per Windows. Prerequisito: Rust da https://rustup.rs
REM (durante l'installazione accetta anche gli "strumenti di compilazione C++ di Visual Studio").
cd /d "%~dp0\..\.."
cargo build --release || exit /b 1
copy /Y target\release\dropall.exe dropall-windows-x86_64.exe
echo.
echo Fatto: %CD%\dropall-windows-x86_64.exe

REM Facoltativo: se è installato Inno Setup 6 (https://jrsoftware.org/isinfo.php) crea anche l'installer.
set "ISCC=%ProgramFiles(x86)%\Inno Setup 6\ISCC.exe"
if not exist "%ISCC%" set "ISCC=%LOCALAPPDATA%\Programs\Inno Setup 6\ISCC.exe"
if exist "%ISCC%" (
  for /f "tokens=3 delims= " %%v in ('findstr /b /c:"version = " Cargo.toml') do set "VER=%%~v"
  "%ISCC%" /DAppVersion=%VER% "/DSourceExe=%CD%\dropall-windows-x86_64.exe" "/O%CD%" packaging\windows\dropall.iss
  echo Installer: %CD%\Dropall-Setup-windows-x86_64.exe
) else (
  echo Per creare anche l'installer installa Inno Setup 6 e rilancia questo file.
)
