@echo off
setlocal EnableExtensions

rem Builds a disposable first-party native notification template, then opens it
rem through Anodrel's fixed three-grant Windows development route. No product
rem identity, installer, certificate, package, or machine policy is created.
set "ANODREL_TEMPLATE=%TEMP%\AnodrelNativeNotificationTemplate-%RANDOM%%RANDOM%"

cd /d "%~dp0"
if errorlevel 1 goto :failed

if not exist "native\Cargo.toml" (
  echo The Anodrel native workspace was not found.
  goto :failed
)

where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust and Cargo are required to start the Anodrel native notification template.
  echo Install Rust from https://rustup.rs/ and run this file again.
  goto :failed
)

echo Building and starting the Anodrel native notification template...
echo The first build takes a moment; then watch for the notification while the window stays open for five seconds.

cargo run --release --manifest-path native\Cargo.toml -p anodrel-native-app-tool -- init-notification "%ANODREL_TEMPLATE%" anodrel-native-notification-template "Anodrel Native Notification Template"
set "ANODREL_EXIT=%ERRORLEVEL%"
if not "%ANODREL_EXIT%"=="0" goto :failed

cargo build --release --manifest-path "%ANODREL_TEMPLATE%\Cargo.toml"
set "ANODREL_EXIT=%ERRORLEVEL%"
if not "%ANODREL_EXIT%"=="0" goto :failed

cargo run --release --manifest-path native\Cargo.toml -p anodrel-windows-host -- --native-notification-template-client "%ANODREL_TEMPLATE%\target\release\anodrel-native-notification-template.exe"
set "ANODREL_EXIT=%ERRORLEVEL%"
if not "%ANODREL_EXIT%"=="0" goto :failed

echo.
echo Native notification template completed successfully.
echo Temporary project retained at: %ANODREL_TEMPLATE%
endlocal
exit /b 0

:failed
echo.
echo Native notification template did not complete. The temporary project, if created, is at: %ANODREL_TEMPLATE%
pause
endlocal
exit /b 1
