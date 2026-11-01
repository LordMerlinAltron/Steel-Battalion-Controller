@echo off
rem Step 3: configure vJoy device 1 and bind the controller to WinUSB (Zadig). Asks for admin once.
net session >nul 2>&1
if errorlevel 1 (
  powershell -NoProfile -Command "Start-Process -FilePath '%~f0' -Verb RunAs"
  exit /b
)
setlocal
cd /d "%~dp0.."
if not exist logs mkdir logs
set "LOG=logs\step3-drivers.txt"
>"%LOG%" echo step3 %DATE% %TIME%

echo [1/3] Configuring vJoy device 1: 8 axes, 80 buttons, no POV ...
set "VJC=%ProgramFiles%\vJoy\x64\vJoyConfig.exe"
if exist "%VJC%" (
  call :run "%VJC%" 1 -f -a x y z rx ry rz sl0 sl1 -b 80 -p 0 -s 0
  call :run "%VJC%" -t 1
) else (
  >>"%LOG%" echo vJoyConfig.exe NOT FOUND at %VJC%
  echo     vJoyConfig.exe not found - open "Configure vJoy" from the Start menu instead.
)

echo.
echo [2/3] Zadig: pick the device with USB ID 0A7B D000, keep WinUSB, click Install Driver.
echo       If the device is missing: Options - List All Devices.
echo       Zadig closes by itself when the install succeeds.
if exist "tools\zadig-2.9.exe" (
  start "" /wait "tools\zadig-2.9.exe"
  >>"%LOG%" echo zadig exit=%ERRORLEVEL%
) else (
  >>"%LOG%" echo tools\zadig-2.9.exe NOT FOUND
  echo     tools\zadig-2.9.exe missing - tell Claude.
)

echo.
echo [3/3] Re-checking drivers ...
powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\p0-check.ps1" >nul
>>"%LOG%" echo p0-check written to logs\p0-check.txt
type "%LOG%"
echo.
echo Done. Tell Claude: "step 3 done"
pause
exit /b

:run
>>"%LOG%" echo.
>>"%LOG%" echo ^> %*
%* >>"%LOG%" 2>&1
>>"%LOG%" echo exit=%ERRORLEVEL%
exit /b
