@echo off
rem Stop the bridge cleanly (vJoy returns to neutral), force-close after 5 s if needed.
setlocal
cd /d "%~dp0.."
tasklist /fi "imagename eq sbc-bridge.exe" 2>nul | find /i "sbc-bridge.exe" >nul
if errorlevel 1 (
  if not "%~1"=="quiet" echo Bridge is not running.
  exit /b 0
)
if not exist bin mkdir bin
type nul > "bin\sbc-bridge.stop"
for /l %%i in (1,1,10) do (
  tasklist /fi "imagename eq sbc-bridge.exe" 2>nul | find /i "sbc-bridge.exe" >nul || goto stopped
  timeout /t 1 /nobreak >nul
)
taskkill /im sbc-bridge.exe /f >nul 2>&1
:stopped
if not "%~1"=="quiet" echo Bridge stopped.
exit /b 0
