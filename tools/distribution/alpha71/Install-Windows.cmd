@echo off
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Install-Windows.ps1"
set "taskSetupExit=%errorlevel%"
if not "%taskSetupExit%"=="0" pause
exit /b %taskSetupExit%
