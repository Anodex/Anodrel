@echo off
setlocal
cd /d "%~dp0"
cargo build --release --manifest-path native\Cargo.toml -p anodrel-native-live-status-event-client
if errorlevel 1 exit /b %errorlevel%
cargo run --release --manifest-path native\Cargo.toml -p anodrel-windows-host -- --uia-live-status-event-probe native\target\release\anodrel-native-live-status-event-client.exe
endlocal
