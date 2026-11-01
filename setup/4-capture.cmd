@echo off
rem Step 4: build sbc-capture and record controller evidence (interactive, ~8 min).
rem Needs: step 2 done, controller bound to WinUSB (step 3).
cd /d "%~dp0.."
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\p1-capture.ps1"
echo.
echo Done. Tell Claude: "step 4 done"
pause
