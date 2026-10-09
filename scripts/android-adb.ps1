# Keep pcsc-rs running on an Android phone, installed from a Windows PC over adb.
# The PowerShell version of android-adb.sh. ASCII only, so Windows PowerShell 5.1
# reads it correctly whether it is run from a file or downloaded with irm.
#
#   $env:PASS = "<PASS>"; .\scripts\android-adb.ps1 install -Hostname my-phone
#   .\scripts\android-adb.ps1 install                  later: keeps PASS and the name
#   .\scripts\android-adb.ps1 install -Binary <file>   install a local build
#   .\scripts\android-adb.ps1 stop | status | log
#   .\scripts\android-adb.ps1 devices                 list connected devices and their serials
#   ... -Adb C:\path\to\adb.exe (or $env:ADB) uses an adb that is not on PATH
#   ... -Serial <serial> (or $env:ANDROID_SERIAL) picks the device: USB, or IP:port for wireless debugging
#
# Straight from GitHub:
#   $env:PASS = "<PASS>"; & ([scriptblock]::Create((irm https://raw.githubusercontent.com/Zel9278/pcsc-rs/main/scripts/android-adb.ps1))) install -Hostname my-phone
#
# Needs adb (Android SDK Platform-Tools) on PATH, or -Adb / $env:ADB pointing at it.
# With several devices, pick one with -Serial or $env:ANDROID_SERIAL (see "devices").
# Same behaviour as android-adb.sh: runs detached, restarts 5 seconds after it exits
# (crash or self-update), survives unplugging and Doze, stops on reboot.
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet("install", "stop", "status", "log", "devices")]
    [string]$Command = "install",
    [string]$Hostname,
    [string]$Binary,
    [int]$Lines = 30,
    [string]$Adb = $(if ($env:ADB) { $env:ADB } else { "adb" }),
    [string]$Serial = $env:ANDROID_SERIAL
)

$ErrorActionPreference = "Stop"
$Dir = "/data/local/tmp/pcsc-rs"
$Target = "aarch64-unknown-linux-musl"

if (-not (Get-Command $Adb -ErrorAction SilentlyContinue)) {
    throw "adb not found: $Adb. Install Android SDK Platform-Tools and add it to PATH, or pass -Adb <path> (or set `$env:ADB): https://developer.android.com/tools/releases/platform-tools"
}

if ($Command -eq "devices") {
    & $Adb devices -l
    return
}

# Pick the device. Without -Serial, use the only one that is connected.
function Get-DeviceList { @(& $Adb devices | Select-Object -Skip 1 | Where-Object { $_ -match "\S+\s+\S+" }) }
# adb writes to stderr when the device is missing; Windows PowerShell 5.1 would turn that into a terminating error under "Stop"
function Get-DeviceState([string]$Id) {
    $ErrorActionPreference = "Continue"
    ((& $Adb -s $Id get-state 2>$null) -join "").Trim()
}
if ($Serial) {
    if ((Get-DeviceState $Serial) -ne "device") {
        throw ("Device $Serial is not available (not connected or not authorized). Connected devices:`n  " + ((Get-DeviceList) -join "`n  "))
    }
} else {
    $list = Get-DeviceList
    $ready = @($list | Where-Object { $_ -match "\s+device$" })
    if ($ready.Count -ne 1) {
        $why = if ($ready.Count -eq 0) { "No usable device is connected (turn on USB or wireless debugging and allow this PC on the device)." } else { "$($ready.Count) devices are connected; pick one with -Serial:" }
        if ($list | Where-Object { $_ -match "unauthorized" }) { $why += " Allow this PC on the screen of the unauthorized ones." }
        throw ($why + $(if ($list) { "`n  " + ($list -join "`n  ") } else { "" }))
    }
}
$SerialArgs = if ($Serial) { @("-s", $Serial) } else { @() }
function Invoke-Adb { & $Adb @SerialArgs @args }

# The remote command goes to adb shell as one string. Windows PowerShell 5.1 mangles
# double quotes inside native arguments, so remote commands never use them.
function Invoke-Remote([string]$RemoteCommand) {
    $output = Invoke-Adb shell $RemoteCommand
    if ($LASTEXITCODE -ne 0) { throw "adb shell failed: $RemoteCommand" }
    $output
}

# Text piped from PowerShell to a native command gets CRLF line endings, so files are
# written locally with LF and UTF-8 (no BOM) and pushed instead.
function Send-File([string]$Text, [string]$RemotePath) {
    $local = Join-Path ([IO.Path]::GetTempPath()) ("pcsc-rs-" + [Guid]::NewGuid().ToString("n"))
    try {
        [IO.File]::WriteAllText($local, ($Text -replace "`r`n", "`n"), (New-Object Text.UTF8Encoding $false))
        Invoke-Adb push $local $RemotePath | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "adb push failed: $RemotePath" }
    } finally {
        Remove-Item -LiteralPath $local -ErrorAction SilentlyContinue
    }
}

function Stop-Remote {
    # Stop the loop first (it writes its own PID to loop.pid), then the client
    Invoke-Remote "test -f $Dir/loop.pid && kill `$(cat $Dir/loop.pid) 2>/dev/null; rm -f $Dir/loop.pid; pkill -x pcsc-rs 2>/dev/null; true" | Out-Null
}

switch ($Command) {
    "install" {
        $arch = ((Invoke-Remote "uname -m") -join "").Trim()
        if ($arch -ne "aarch64") { throw "Unsupported architecture (aarch64 only): $arch" }

        $download = $null
        if (-not $Binary) {
            [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
            $tag = (Invoke-RestMethod -Uri "https://api.github.com/repos/Zel9278/pcsc-rs/releases/latest" -UseBasicParsing).tag_name
            if (-not $tag) { throw "Could not get the latest release" }
            $download = Join-Path ([IO.Path]::GetTempPath()) "pcsc-rs-$tag-$Target"
            Invoke-WebRequest -Uri "https://github.com/Zel9278/pcsc-rs/releases/download/$tag/pcsc-rs-$tag-$Target" -OutFile $download -UseBasicParsing
            $Binary = $download
        }

        try {
            Invoke-Remote "mkdir -p $Dir" | Out-Null
            # Without -Hostname or PASS, keep the ones in the installed .env
            $oldEnv = @(Invoke-Adb shell "cat $Dir/.env 2>/dev/null") | ForEach-Object { "$_".TrimEnd("`r") }
            $pass = if ($env:PASS) { $env:PASS } else { ($oldEnv | Where-Object { $_ -like "PASS=*" } | Select-Object -First 1) -replace "^PASS=", "" }
            if (-not $Hostname) { $Hostname = ($oldEnv | Where-Object { $_ -like "HOSTNAME=*" } | Select-Object -First 1) -replace "^HOSTNAME=", "" }
            if (-not $pass) { throw "Set PASS first: `$env:PASS = `"<PASS>`"" }

            $envText = "PASS=$pass`nPCSC_UPDATED=terminate`n"
            if ($Hostname) { $envText += "HOSTNAME=$Hostname`n" }

            Stop-Remote
            Invoke-Adb push $Binary "$Dir/pcsc-rs" | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "adb push failed" }
            Send-File $envText "$Dir/.env"
            # Restart the client 5 seconds after it exits (crash or self-update). .env is read from the working directory.
            Send-File @'
#!/system/bin/sh
cd "$(dirname "$0")" || exit 1
echo $$ > loop.pid
# adb shell sets HOSTNAME to the device codename; use the name in .env (or the model) instead
unset HOSTNAME
while true; do
  ./pcsc-rs
  sleep 5
done
'@ "$Dir/loop.sh"
            Invoke-Remote "chmod 600 $Dir/.env; chmod 755 $Dir/pcsc-rs $Dir/loop.sh" | Out-Null
            # Start from a subshell so the adb shell process exits at once (otherwise adb never returns)
            Invoke-Remote "cd $Dir && (setsid $Dir/loop.sh > pcsc-rs.log 2>&1 < /dev/null &)" | Out-Null
            Start-Sleep -Seconds 4
            Invoke-Remote "tail -n 6 $Dir/pcsc-rs.log"
        } finally {
            if ($download) { Remove-Item -LiteralPath $download -ErrorAction SilentlyContinue }
        }
    }
    "stop" {
        Stop-Remote
        "Stopped"
    }
    "status" {
        $ps = @(Invoke-Adb shell "ps -A -o PID,USER,ETIME,ARGS | grep -E '^ *PID|pcsc-rs' | grep -v grep")
        if ($ps.Count -gt 1) { $ps } else { "Not running" }
    }
    "log" {
        Invoke-Remote "tail -n $Lines $Dir/pcsc-rs.log"
    }
}
