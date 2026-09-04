@echo off
setlocal EnableExtensions

rem Builds a disposable first-party native file-write template, then opens it
rem through Anodrel's fixed retained-output Windows development route. No product
rem identity, installer, certificate, package, or machine policy is created.
set "ANODREL_TEMPLATE=%TEMP%\AnodrelNativeFileWriteTemplate-%RANDOM%%RANDOM%"

cd /d "%~dp0"
if errorlevel 1 goto :failed

if not exist "native\Cargo.toml" (
  echo The Anodrel native workspace was not found.
  goto :failed
)

where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust and Cargo are required to start the Anodrel native file-write template.
  echo Install Rust from https://rustup.rs/ and run this file again.
  goto :failed
)

echo Building and starting the Anodrel native file-write template...
echo The first build takes a moment; then choose a fresh .txt destination in the save dialog.

cargo run --release --manifest-path native\Cargo.toml -p anodrel-native-app-tool -- init-file-write "%ANODREL_TEMPLATE%" anodrel-native-file-write-template "Anodrel Native File Write Template"
set "ANODREL_EXIT=%ERRORLEVEL%"
if not "%ANODREL_EXIT%"=="0" goto :failed

cargo build --release --manifest-path "%ANODREL_TEMPLATE%\Cargo.toml"
set "ANODREL_EXIT=%ERRORLEVEL%"
if not "%ANODREL_EXIT%"=="0" goto :failed

cargo run --release --manifest-path native\Cargo.toml -p anodrel-windows-host -- --native-file-write-template-client "%ANODREL_TEMPLATE%\target\release\anodrel-native-file-write-template.exe"
set "ANODREL_EXIT=%ERRORLEVEL%"
if not "%ANODREL_EXIT%"=="0" goto :failed

echo.
echo Native file-write template completed successfully.
echo Inspect the selected file for the documented fixed line.
echo Temporary project retained at: %ANODREL_TEMPLATE%
endlocal
exit /b 0

:failed
echo.
echo Native file-write template did not complete. The temporary project, if created, is at: %ANODREL_TEMPLATE%
pause
endlocal
exit /b 1
