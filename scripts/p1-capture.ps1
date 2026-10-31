# P1 hardware capture. Requires: controller bound to WinUSB (Zadig), Rust + MSVC build tools.
# Normal (non-admin) PowerShell is fine:
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\p1-capture.ps1
# Output: logs\p1-*.txt, fixtures\local\idle\*.bin

param(
    [ValidateRange(30, 1800)] [int]$WatchSecs = 300,
    [ValidateRange(300, 5000)] [int]$DwellMs = 1000
)

$ErrorActionPreference = 'Stop'
$root   = Split-Path -Parent $PSScriptRoot
$logDir = Join-Path $root 'logs'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
Set-Location $root

function Invoke-Logged([string]$name, [scriptblock]$body) {
    $log = Join-Path $logDir "p1-$name.txt"
    $ErrorActionPreference = 'Continue'   # native stderr (cargo progress) must not throw
    & $body 2>&1 | ForEach-Object { "$_" } | Tee-Object -FilePath $log
    if ($LASTEXITCODE -ne 0) { throw "step '$name' failed (exit $LASTEXITCODE), see $log" }
}

Write-Host '[1/5] build (whole workspace: capture tool + bridge)' -ForegroundColor Cyan
Invoke-Logged 'build' { cargo build --workspace --release }
$exe = Join-Path $root 'target\release\sbc-capture.exe'
if (-not (Test-Path $exe)) { throw "missing $exe" }

Write-Host '[2/5] USB descriptors' -ForegroundColor Cyan
Invoke-Logged 'info' { & $exe info }

Write-Host '[3/5] idle packets (hands off the controller, 5 s)' -ForegroundColor Cyan
Invoke-Logged 'raw-idle' { & $exe raw --secs 5 --max 50 --save (Join-Path $root 'fixtures\local\idle') }

Write-Host '[4/5] LED probe: watch the panel. Note any line where the WRONG lamp lights, or none.' -ForegroundColor Yellow
Read-Host 'Press Enter to start the LED probe'
Invoke-Logged 'led-probe' { & $exe led-probe --dwell $DwellMs }
$ledNotes = Read-Host 'LED mismatches (e.g. "id 14 lit Washing") or Enter if all correct'
Set-Content -Encoding UTF8 -Path (Join-Path $logDir 'p1-led-notes.txt') -Value $ledNotes

Write-Host "[5/5] watch ($WatchSecs s). Do in order:" -ForegroundColor Yellow
Write-Host '  1. every button once, left to right, top to bottom'
Write-Host '  2. each toggle switch on then off'
Write-Host '  3. gear lever R, N, 1, 2, 3, 4, 5'
Write-Host '  4. tuner dial one full turn clockwise'
Write-Host '  5. each stick to all 4 edges; each pedal to the floor'
Read-Host 'Press Enter to start'
Invoke-Logged 'watch' { & $exe watch --secs $WatchSecs }

Write-Host "`nDone. Logs in $logDir" -ForegroundColor Green
