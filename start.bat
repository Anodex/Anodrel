@echo off
setlocal EnableExtensions

cd /d "%~dp0"

if not exist "apps\sample\anodrel.application.json" (
  echo Anodrel sample package was not found.
  goto :failure
)

where cargo >nul 2>nul
if errorlevel 1 (
  echo Rust and Cargo are required to start the Anodrel sample.
  echo Install Rust from https://rustup.rs/ and run this file again.
  goto :failure
)

echo Building and starting the Anodrel Startup Lab...
echo The first build takes a moment; later runs start immediately.
rem Release is required, not a preference: the Startup Lab composes every frame
rem in software, and an unoptimised build cannot hold the reveal's frame rate.
cargo run --release --manifest-path native\Cargo.toml -p anodrel-windows-host -- --showcase apps\sample\anodrel.application.json
set "exit_code=%ERRORLEVEL%"

if not "%exit_code%"=="0" (
  echo.
  echo Anodrel stopped with exit code %exit_code%.
  goto :failure
)

endlocal
exit /b 0

:failure
pause
endlocal
exit /b 1
