@echo off
rem Step 4b: measure stick, lever and pedal ranges -> config\calibration.toml. No admin. ~1 min. Safe to re-run.
setlocal
cd /d "%~dp0.."
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
if not exist logs mkdir logs
set "LOG=logs\step4b-calibrate.txt"

echo [1/3] Building sbc-capture ...
cargo build --release -p sbc-capture >"%LOG%" 2>&1
if errorlevel 1 goto fail

echo.
echo [2/3] Calibrate. After you press a key:
echo   - first 3 s: HANDS AND FEET OFF (rest position). The aim stick can stay wherever it is.
echo   - then 40 s: aim stick and sight stick slowly around all edges (aim stick: all 4 corners),
echo                rotation lever fully both ways, each pedal to the floor once.
pause
"target\release\sbc-capture.exe" calibrate --secs 40 --out config\calibration.toml
if errorlevel 1 goto fail
>>"%LOG%" type config\calibration.toml

echo.
echo [3/3] Applying ...
if not exist "bin\sbc-bridge.exe" goto notinstalled
copy /y "config\calibration.toml" "bin\sbc-calibration.toml" >nul
call "%~dp0bridge-stop.cmd" quiet
call "%~dp0bridge-start.cmd" quiet
echo Bridge restarted with the new calibration.
goto done
:notinstalled
echo Bridge not installed yet; step 5 will use config\calibration.toml.
:done
echo.
echo Done. Tell Claude: "step 4b done"
pause
exit /b 0

:fail
type "%LOG%"
echo.
echo FAILED. Tell Claude: "step 4b failed"
pause
exit /b 1
