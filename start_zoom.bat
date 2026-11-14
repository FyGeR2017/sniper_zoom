@echo off
cd /d "%~dp0"
if exist "target\release\sniper_zoom.exe" (
    start "" "target\release\sniper_zoom.exe"
    exit
) else (
    cargo build --release --bin sniper_zoom
    start "" "target\release\sniper_zoom.exe"
    exit
)
