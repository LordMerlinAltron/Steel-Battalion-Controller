@echo off
rem Stop the bridge and remove it from Startup. Leaves bin\ and logs\ in place.
setlocal
call "%~dp0bridge-stop.cmd" quiet
powershell -NoProfile -Command "Remove-Item -LiteralPath ([Environment]::GetFolderPath('Startup')+'\Steel Battalion Bridge.lnk') -ErrorAction SilentlyContinue"
echo Bridge stopped and removed from Startup.
pause
