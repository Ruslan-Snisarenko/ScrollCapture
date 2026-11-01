@echo off
cd /d "%~dp0"
echo Сборка ScrollCapture...
cargo build --release
if %ERRORLEVEL% EQU 0 (
    copy /Y "target\release\scrollcapture_rust.exe" "ScrollCapture.exe" > nul
    echo Сборка успешно завершена!
) else (
    echo Ошибка сборки!
)
pause
