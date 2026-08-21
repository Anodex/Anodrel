@echo off
setlocal EnableExtensions

rem Builds and runs the one fixed-origin native HTTPS diagnostic. It grants no
rem general network access: the host permits only example.com:443, and the
rem first-party child requests only https://example.com/.
cd /d "%~dp0"
if errorlevel 1 goto :failed

if not exist "native\Cargo.toml" (
  echo The Anodrel native workspace was not found.
  goto :failed
)

where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust and Cargo are required to start the Anodrel HTTPS diagnostic.
  echo Install Rust from https://rustup.rs/ and run this file again.
  goto :failed
)

echo Building and running the Anodrel native HTTPS diagnostic...
echo It needs ordinary outbound Internet access and opens no window.
cargo build --release --manifest-path native\Cargo.toml -p anodrel-native-network-client-sample -p anodrel-windows-host
set "ANODREL_EXIT=%ERRORLEVEL%"
if not "%ANODREL_EXIT%"=="0" goto :failed

native\target\release\anodrel-windows-host.exe --native-network-sample-client "native\target\release\anodrel-native-network-client-sample.exe"
set "ANODREL_EXIT=%ERRORLEVEL%"
if not "%ANODREL_EXIT%"=="0" goto :failed

echo.
echo Native HTTPS diagnostic completed successfully.
endlocal
exit /b 0

:failed
echo.
echo Native HTTPS diagnostic did not complete. No certificate, proxy, or machine policy was changed.
pause
endlocal
exit /b 1
