# Install the latest pcsc-rs release on Windows and keep it running with Task Scheduler.
# ASCII only, so Windows PowerShell 5.1 reads it correctly whether it is run from a
# file or downloaded with irm.
#
#   $env:PASS = "<PASS>"; & ([scriptblock]::Create((irm https://raw.githubusercontent.com/Zel9278/pcsc-rs/main/install.ps1)))
#       for this user (%LOCALAPPDATA%\pcsc-rs, starts at logon)
#   ... -System   (in an administrator PowerShell)
#       for the whole PC (%ProgramFiles%\pcsc-rs, starts at boot as SYSTEM, runs without anyone logged on)
#   ... -Hostname my-pc   name shown on PC Status
#   ... -Uninstall        stop and remove it
#
# Run it again without PASS to update: PASS and the other settings in .env (HOSTNAME, ...) are kept.
# pcsc-rs updates itself (PCSC_UPDATED=restart: after replacing itself it starts the new version and exits).
[CmdletBinding()]
param(
    [switch]$System,
    [string]$Hostname,
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"
$Repo = "Zel9278/pcsc-rs"
# There is no ARM64 build, but Windows on ARM runs x64
$Target = "x86_64-pc-windows-msvc"

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
if ($System) {
    if (-not $isAdmin) { throw "-System needs an administrator PowerShell" }
    $Dir = Join-Path $env:ProgramFiles "pcsc-rs"
    $TaskName = "pcsc-rs"
    $OtherTask = "pcsc-rs ($env:USERNAME)"
} else {
    $Dir = Join-Path $env:LOCALAPPDATA "pcsc-rs"
    $TaskName = "pcsc-rs ($env:USERNAME)"
    $OtherTask = "pcsc-rs"
}
$Exe = Join-Path $Dir "pcsc-rs.exe"
$EnvFile = Join-Path $Dir ".env"

function Stop-Client {
    if (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue) {
        Stop-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    }
    # A client restarted by PCSC_UPDATED=restart runs outside the task; find it by its path
    Get-Process -Name pcsc-rs -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and $_.Path -ieq $Exe } |
        Stop-Process -Force -ErrorAction SilentlyContinue
    Start-Sleep -Milliseconds 500
}

if ($Uninstall) {
    Stop-Client
    if (Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue) {
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
    }
    if (Test-Path -LiteralPath $Dir) { Remove-Item -LiteralPath $Dir -Recurse -Force }
    "Removed $Dir and the task '$TaskName'"
    return
}

# Only one client per hostname can connect
if (Get-ScheduledTask -TaskName $OtherTask -ErrorAction SilentlyContinue) {
    Write-Warning "The other install (task '$OtherTask') is there too. With the same hostname the server refuses whichever connects second; -Uninstall one of them."
}

# Keep PASS and the other settings from the installed .env
$keep = [ordered]@{}
if (Test-Path -LiteralPath $EnvFile) {
    foreach ($line in Get-Content -LiteralPath $EnvFile -Encoding UTF8) {
        if ($line -match "^\s*([A-Za-z_][A-Za-z0-9_]*)=(.*)$") {
            $key = $Matches[1]; $value = $Matches[2]
            # Written in quotes by this script; older versions wrote them bare
            if ($value -match "^'(.*)'$" -or $value -match '^"(.*)"$') { $value = $Matches[1] }
            $keep[$key] = $value
        }
    }
}
$pass = if ($env:PASS) { $env:PASS } else { $keep["PASS"] }
if (-not $pass) { throw "Set PASS first: `$env:PASS = `"<PASS>`"" }
$keep.Remove("PASS")
$keep.Remove("PCSC_UPDATED")
if ($Hostname) { $keep["HOSTNAME"] = $Hostname }

# ---- Download ----
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$tag = (Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing).tag_name
if (-not $tag) { throw "Could not get the latest release" }
$download = Join-Path ([IO.Path]::GetTempPath()) "pcsc-rs-$tag-$Target.exe"
Invoke-WebRequest -Uri "https://github.com/$Repo/releases/download/$tag/pcsc-rs-$tag-$Target.exe" -OutFile $download -UseBasicParsing
Unblock-File -LiteralPath $download

try {
    New-Item -ItemType Directory -Force -Path $Dir | Out-Null
    Stop-Client
    Copy-Item -LiteralPath $download -Destination $Exe -Force
} finally {
    Remove-Item -LiteralPath $download -ErrorAction SilentlyContinue
}

# .env in UTF-8 without BOM; pcsc-rs reads it from its working directory.
# Values go in single quotes, so spaces and symbols are read as they are; a quote cannot be.
foreach ($value in @($pass) + @($keep.Values)) {
    if ("$value".Contains("'")) { throw "Settings cannot contain ': $value" }
}
$lines = @("PASS='$pass'", "PCSC_UPDATED=restart") + @($keep.GetEnumerator() | ForEach-Object { "$($_.Key)='$($_.Value)'" })
[IO.File]::WriteAllText($EnvFile, (($lines -join "`n") + "`n"), (New-Object Text.UTF8Encoding $false))
if (-not $System) {
    # Only this user may read it (it holds PASS)
    icacls $EnvFile /inheritance:r /grant:r "${env:USERNAME}:(F)" | Out-Null
}

# ---- Register with Task Scheduler and start ----
$action = New-ScheduledTaskAction -Execute $Exe -WorkingDirectory $Dir
if ($System) {
    $trigger = New-ScheduledTaskTrigger -AtStartup
    $principal = New-ScheduledTaskPrincipal -UserId "SYSTEM" -LogonType ServiceAccount -RunLevel Highest
} else {
    $trigger = New-ScheduledTaskTrigger -AtLogOn -User "$env:USERDOMAIN\$env:USERNAME"
    $principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" -LogonType Interactive -RunLevel Limited
}
# Drop the defaults "stop after 3 days" and "not on battery"; restart a minute after a failure
$settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit ([TimeSpan]::Zero) -RestartCount 999 -RestartInterval (New-TimeSpan -Minutes 1) `
    -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -MultipleInstances IgnoreNew
Register-ScheduledTask -TaskName $TaskName -Action $action -Trigger $trigger -Principal $principal -Settings $settings `
    -Description "PC Status client (https://github.com/$Repo)" -Force | Out-Null
Start-ScheduledTask -TaskName $TaskName
Start-Sleep -Seconds 3

$running = Get-Process -Name pcsc-rs -ErrorAction SilentlyContinue | Where-Object { $_.Path -ieq $Exe }
if ($running) {
    "Installed $tag in $Dir (task '$TaskName'), running"
} else {
    Write-Warning "Installed $tag in $Dir (task '$TaskName'), but it is not running yet; check the task in Task Scheduler"
}
