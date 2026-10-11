@echo off
setlocal
set "SYNDOCAL_POWERSHELL=%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe"
if exist "%SystemRoot%\Sysnative\WindowsPowerShell\v1.0\powershell.exe" set "SYNDOCAL_POWERSHELL=%SystemRoot%\Sysnative\WindowsPowerShell\v1.0\powershell.exe"
"%SYNDOCAL_POWERSHELL%" -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Uninstall-Windows.ps1"
set "taskUninstallExit=%errorlevel%"
if not "%taskUninstallExit%"=="0" pause
exit /b %taskUninstallExit%
