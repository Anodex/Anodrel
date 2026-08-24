@echo off
setlocal
cd /d "%~dp0"
cargo run --release --manifest-path native\Cargo.toml -p anodrel-windows-host -- --uia-property-probe
endlocal
