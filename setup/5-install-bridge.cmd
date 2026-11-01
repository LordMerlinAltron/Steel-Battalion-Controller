@echo off
rem Step 5: build sbc-bridge, install it to bin\, start it now and at every logon (hidden). No admin. Safe to re-run.
setlocal
cd /d "%~dp0.."
set "ROOT=%CD%"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
if not exist logs mkdir logs
set "LOG=logs\step5-install.txt"
>"%LOG%" echo step5 %DATE% %TIME%

echo [1/4] Stopping a running bridge, if any ...
call "%~dp0bridge-stop.cmd" quiet

echo [2/4] Building (first time takes a few minutes) ...
call :run cargo build --release -p sbc-bridge
if errorlevel 1 goto fail

echo [3/4] Installing to bin\ and adding to Startup ...
if not exist bin mkdir bin
call :run copy /y "target\release\sbc-bridge.exe" "bin\sbc-bridge.exe"
if errorlevel 1 goto fail
if not exist "bin\sbc-bridge.toml" call :run copy /y "config\default.toml" "bin\sbc-bridge.toml"
if exist "config\calibration.toml" call :run copy /y "config\calibration.toml" "bin\sbc-calibration.toml"
call :run powershell -NoProfile -Command "$s=(New-Object -ComObject WScript.Shell).CreateShortcut([Environment]::GetFolderPath('Startup')+'\Steel Battalion Bridge.lnk'); $s.TargetPath='%ROOT%\bin\sbc-bridge.exe'; $s.Arguments='--log \"%ROOT%\logs\bridge.log\"'; $s.WorkingDirectory='%ROOT%\bin'; $s.Description='Steel Battalion controller to vJoy'; $s.Save()"
if errorlevel 1 goto fail

echo [4/4] Starting ...
call "%~dp0bridge-start.cmd" quiet
powershell -NoProfile -Command "Start-Sleep 3; Get-Content '%ROOT%\logs\bridge.log' -Tail 12" >>"%LOG%" 2>&1
type "%LOG%"
echo.
echo Done. The bridge now runs hidden and starts at logon. Tell Claude: "step 5 done"
pause
exit /b 0

:fail
type "%LOG%"
echo.
echo FAILED. Tell Claude: "step 5 failed"
pause
exit /b 1

:run
>>"%LOG%" echo.
>>"%LOG%" echo ^> %*
%* >>"%LOG%" 2>&1
set "RC=%ERRORLEVEL%"
>>"%LOG%" echo exit=%RC%
exit /b %RC%
