# P0 read-only system check for the Steel Battalion bridge.
# Changes nothing. Run from an elevated (Administrator) PowerShell:
#   powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\p0-check.ps1
# Output: logs\p0-check.txt

$ErrorActionPreference = 'Continue'
$root   = Split-Path -Parent $PSScriptRoot
$logDir = Join-Path $root 'logs'
$log    = Join-Path $logDir 'p0-check.txt'
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

function Write-Section([string]$title) { "`n=== $title ===" }

function Get-SbcDevices {
    Get-PnpDevice -PresentOnly -ErrorAction SilentlyContinue |
        Where-Object { $_.InstanceId -like '*VID_0A7B&PID_D000*' }
}

& {
    Write-Section 'meta'
    Get-Date -Format o
    $os = Get-CimInstance Win32_OperatingSystem
    "$($os.Caption) build $($os.BuildNumber)"
    $id = [Security.Principal.WindowsIdentity]::GetCurrent()
    $admin = ([Security.Principal.WindowsPrincipal]$id).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    "admin=$admin"

    Write-Section 'secure boot'
    try { "SecureBoot=$(Confirm-SecureBootUEFI)" } catch { "SecureBoot=unknown ($($_.Exception.Message))" }

    Write-Section 'test signing (needs admin)'
    $bcd = bcdedit /enum '{current}' 2>&1 | Out-String
    $flags = $bcd -split "`r?`n" | Where-Object { $_ -match 'testsigning|nointegritychecks' }
    if ($flags) { $flags } else { 'testsigning/nointegritychecks not set (good)' }

    Write-Section '2019 test-signed driver'
    $old = pnputil /enum-drivers 2>&1 | Select-String -Context 1,6 -Pattern 'hidsteelbattalion|steelbattalion'
    if ($old) { $old | Out-String } else { 'not installed (good)' }

    Write-Section 'controller 0A7B:D000'
    $sbc = @(Get-SbcDevices)
    if ($sbc.Count -eq 0) { 'NOT FOUND (plugged in? powered?)' }
    foreach ($d in $sbc) {
        $svc = (Get-PnpDeviceProperty -InstanceId $d.InstanceId -KeyName 'DEVPKEY_Device_Service' -ErrorAction SilentlyContinue).Data
        "status=$($d.Status) class=$($d.Class) service=$svc name=$($d.FriendlyName)"
        "instance=$($d.InstanceId)"
    }

    Write-Section 'vJoy'
    $vj = @(Get-PnpDevice -FriendlyName '*vJoy*' -ErrorAction SilentlyContinue)
    if ($vj.Count -eq 0) { 'vJoy device NOT FOUND' }
    foreach ($d in $vj) { "status=$($d.Status) problem=$($d.Problem) name=$($d.FriendlyName)" }
    $dll = Join-Path $env:ProgramFiles 'vJoy\x64\vJoyInterface.dll'
    "vJoyInterface.dll present=$(Test-Path $dll)"
    $keys = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\*',
            'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
    Get-ItemProperty $keys -ErrorAction SilentlyContinue |
        Where-Object { $_.DisplayName -like '*vJoy*' } |
        ForEach-Object { "installed: $($_.DisplayName) $($_.DisplayVersion)" }

    Write-Section 'tools'
    foreach ($cmd in 'git', 'cargo', 'rustc', 'rustup', 'winget') {
        $c = Get-Command $cmd -ErrorAction SilentlyContinue
        if ($c) { "$cmd : $((& $cmd --version 2>&1 | Select-Object -First 1))" } else { "$cmd : MISSING" }
    }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (Test-Path $vswhere) {
        $vc = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        if ($vc) { "MSVC build tools: $vc" } else { 'MSVC build tools: MISSING (C++ workload)' }
    } else { 'MSVC build tools: MISSING (no Visual Studio installer)' }
} *>&1 | Out-String -Width 220 | Set-Content -Encoding UTF8 -Path $log

Get-Content $log
Write-Host "`nSaved: $log"
