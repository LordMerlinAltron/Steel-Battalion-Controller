@echo off
rem Step 2: install Rust (MSVC toolchain, same as CI) + Visual Studio C++ Build Tools. Safe to re-run.
rem The Build Tools install shows one UAC prompt and takes ~10-20 min (~4-6 GB).
setlocal
cd /d "%~dp0.."
if not exist logs mkdir logs
set "LOG=logs\step2-rust.txt"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
echo step2 %DATE% %TIME% > "%LOG%"

where rustup >nul 2>&1
if errorlevel 1 call :run winget install --id Rustlang.Rustup -e --silent --accept-package-agreements --accept-source-agreements
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

set "VSWHERE=%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe"
call :findvc
if defined HAVE_VC goto vcok
echo Installing Visual Studio C++ Build Tools. Click Yes on the admin prompt, then wait 10-20 min ...
call :run winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-package-agreements --accept-source-agreements --override "--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
call :findvc
:vcok
>>"%LOG%" echo.
>>"%LOG%" echo MSVC build tools: "%HAVE_VC%"

call :run rustup set default-host x86_64-pc-windows-msvc
call :run rustup toolchain install stable-x86_64-pc-windows-msvc --profile minimal
call :run rustup default stable-x86_64-pc-windows-msvc
call :run rustup toolchain list
call :run cargo --version
call :run rustc -vV

type "%LOG%"
echo.
if not defined HAVE_VC goto bad
findstr /c:"host: x86_64-pc-windows-msvc" "%LOG%" >nul
if errorlevel 1 goto bad
echo OK: MSVC toolchain and C++ Build Tools ready. Tell Claude: "step 2 done"
pause
exit /b
:bad
echo PROBLEM: toolchain or C++ Build Tools missing. Tell Claude: "step 2 failed"
pause
exit /b

:run
>>"%LOG%" echo.
>>"%LOG%" echo ^> %*
%* >>"%LOG%" 2>&1
>>"%LOG%" echo exit=%ERRORLEVEL%
exit /b

:findvc
set "HAVE_VC="
if not exist "%VSWHERE%" exit /b
for /f "usebackq delims=" %%i in (`"%VSWHERE%" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "HAVE_VC=%%i"
exit /b
