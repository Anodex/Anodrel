@echo off
setlocal
cd /d "%~dp0"
cargo build --release --manifest-path native\Cargo.toml -p anodrel-native-structure-event-client
if errorlevel 1 exit /b %errorlevel%
cargo run --release --manifest-path native\Cargo.toml -p anodrel-windows-host -- --uia-structure-event-probe native\target\release\anodrel-native-structure-event-client.exe
endlocal
