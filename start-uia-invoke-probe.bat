@echo off
setlocal
cd /d "%~dp0"
cargo build --release --manifest-path native\Cargo.toml -p anodrel-native-ui-client-sample
if errorlevel 1 exit /b %errorlevel%
cargo run --release --manifest-path native\Cargo.toml -p anodrel-windows-host -- --uia-invoke-probe native\target\release\anodrel-native-ui-client-sample.exe
endlocal
