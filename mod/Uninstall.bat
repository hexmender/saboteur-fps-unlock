@echo off
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0SaboteurFpsUnlock.ps1" -Uninstall %*
pause
