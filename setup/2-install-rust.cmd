@echo off
rem Step 2: install Rust (user-level, GNU toolchain: no Visual Studio needed).
cd /d "%~dp0.."
if not exist logs mkdir logs
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
where rustup >nul 2>&1
if errorlevel 1 (
  winget install --id Rustlang.Rustup -e --silent --accept-package-agreements --accept-source-agreements
)
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
rustup set default-host x86_64-pc-windows-gnu
rustup toolchain install stable-x86_64-pc-windows-gnu --profile minimal
rustup default stable-x86_64-pc-windows-gnu
(rustup show & cargo --version & rustc -vV) > logs\step2-rust.txt 2>&1
type logs\step2-rust.txt
echo.
echo Done. Tell Claude: "step 2 done"
pause
