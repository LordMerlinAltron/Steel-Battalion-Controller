@echo off
rem Start the installed bridge (hidden). Log: logs\bridge.log
setlocal
cd /d "%~dp0.."
if not exist "bin\sbc-bridge.exe" (
  echo Not installed. Run setup\5-install-bridge.cmd first.
  if not "%~1"=="quiet" pause
  exit /b 1
)
start "" "%CD%\bin\sbc-bridge.exe" --log "%CD%\logs\bridge.log"
if not "%~1"=="quiet" (
  echo Bridge started. Log: %CD%\logs\bridge.log
  timeout /t 3 >nul
)
exit /b 0
