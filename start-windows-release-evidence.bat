@echo off
setlocal
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File scripts\verify-windows-release.ps1 -IncludeIdleReport -IncludeAccessibilityReport
set "exitCode=%ERRORLEVEL%"
endlocal & exit /b %exitCode%
