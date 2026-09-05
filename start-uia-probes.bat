@echo off
setlocal
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\verify-windows-accessibility.ps1
set "exitCode=%ERRORLEVEL%"
endlocal & exit /b %exitCode%
