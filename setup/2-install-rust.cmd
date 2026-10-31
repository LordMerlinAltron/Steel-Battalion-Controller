@echo off
rem Step 2: install Rust (user-level, GNU toolchain: no Visual Studio needed). Safe to re-run.
setlocal
cd /d "%~dp0.."
if not exist logs mkdir logs
set "LOG=logs\step2-rust.txt"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
echo step2 %DATE% %TIME% > "%LOG%"

where rustup >nul 2>&1
if errorlevel 1 call :run winget install --id Rustlang.Rustup -e --silent --accept-package-agreements --accept-source-agreements
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

call :run rustup set default-host x86_64-pc-windows-gnu
call :run rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal
call :run rustup default stable-x86_64-pc-windows-gnu
call :run rustup toolchain list
call :run cargo --version
call :run rustc -vV

type "%LOG%"
echo.
findstr /c:"host: x86_64-pc-windows-gnu" "%LOG%" >nul
if errorlevel 1 (
  echo PROBLEM: toolchain is not GNU. Tell Claude: "step 2 failed"
) else (
  echo OK: GNU toolchain active. Tell Claude: "step 2 done"
)
pause
exit /b

:run
>>"%LOG%" echo.
>>"%LOG%" echo ^> %*
%* >>"%LOG%" 2>&1
>>"%LOG%" echo exit=%ERRORLEVEL%
exit /b
