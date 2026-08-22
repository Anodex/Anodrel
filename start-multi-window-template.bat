@echo off
setlocal EnableExtensions

rem Builds a disposable first-party native multi-window template, then opens it
rem through Anodrel's fixed five-grant Windows development route. No product
rem identity, installer, certificate, package, or machine policy is created.
set "ANODREL_TEMPLATE=%TEMP%\AnodrelNativeMultiWindowTemplate-%RANDOM%%RANDOM%"

cd /d "%~dp0"
if errorlevel 1 goto :failed

if not exist "native\Cargo.toml" (
  echo The Anodrel native workspace was not found.
  goto :failed
)

where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust and Cargo are required to start the Anodrel native multi-window template.
  echo Install Rust from https://rustup.rs/ and run this file again.
  goto :failed
)

echo Building and starting the Anodrel native multi-window template...
echo The first build takes a moment; then open the secondary window and complete its two actions.

cargo run --release --manifest-path native\Cargo.toml -p anodrel-native-app-tool -- init-multi-window "%ANODREL_TEMPLATE%" anodrel-native-multi-window-template "Anodrel Native Multi-Window Template"
set "ANODREL_EXIT=%ERRORLEVEL%"
if not "%ANODREL_EXIT%"=="0" goto :failed

cargo build --release --manifest-path "%ANODREL_TEMPLATE%\Cargo.toml"
set "ANODREL_EXIT=%ERRORLEVEL%"
if not "%ANODREL_EXIT%"=="0" goto :failed

cargo run --release --manifest-path native\Cargo.toml -p anodrel-windows-host -- --native-multi-window-template-client "%ANODREL_TEMPLATE%\target\release\anodrel-native-multi-window-template.exe"
set "ANODREL_EXIT=%ERRORLEVEL%"
if not "%ANODREL_EXIT%"=="0" goto :failed

echo.
echo Native multi-window template completed successfully.
echo Temporary project retained at: %ANODREL_TEMPLATE%
endlocal
exit /b 0

:failed
echo.
echo Native multi-window template did not complete. The temporary project, if created, is at: %ANODREL_TEMPLATE%
pause
endlocal
exit /b 1
