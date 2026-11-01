@echo off
rem Step 1: read-only system check. Changes nothing. Asks for admin (UAC) once.
net session >nul 2>&1
if errorlevel 1 (
  powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
  exit /b
)
cd /d "%~dp0.."
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\p0-check.ps1"
echo.
echo Done. Tell Claude: "step 1 done"
pause
