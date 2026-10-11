# Keep pcsc-rs running on an Android phone, installed from a Windows PC over adb.

# The PowerShell version of android-adb.sh. ASCII only, so Windows PowerShell 5.1

# reads it correctly when run from a file.

#

#   $env:PASS = "<PASS>"; .\scripts\android-adb.ps1 install -Hostname my-phone

#   .\scripts\android-adb.ps1 install                  installs the matching local build

#   .\scripts\android-adb.ps1 install -Binary <file>   install a specific local build

#   .\scripts\android-adb.ps1 stop | status | log

#   .\scripts\android-adb.ps1 devices                 list connected devices and their serials

#   ... -Adb C:\path\to\adb.exe (or $env:ADB) uses an adb that is not on PATH

#   ... -Serial <serial> (or $env:ANDROID_SERIAL) picks the device: USB, or IP:port for wireless debugging

#

# Needs adb (Android SDK Platform-Tools) on PATH, or -Adb / $env:ADB pointing at it.

# With several devices, pick one with -Serial or $env:ANDROID_SERIAL (see "devices").

# Same behaviour as android-adb.sh: runs detached, restarts 5 seconds after it exits

# (crash), survives unplugging where the device permits it, and stops on reboot.

[CmdletBinding()]

param(

    [Parameter(Position = 0)]

    [ValidateSet("install", "stop", "status", "log", "devices")]

    [string]$Command = "install",

    [string]$Hostname,

    [string]$Binary,

    [int]$Lines = 30,

    [string]$Adb = $(if ($env:ADB) { $env:ADB } else { "adb" }),

    [string]$Serial = $env:ANDROID_SERIAL,

    [ValidateSet("auto", "bionic", "musl")]

    [string]$Libc = "auto"

)

$ErrorActionPreference = "Stop"

$Dir = "/data/local/tmp/pcsc-rs"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

# The Android build (bionic) uses Android's DNS resolver. Older releases without it get the musl

# build, which looks for /etc/resolv.conf and cannot resolve names on some devices.

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

# A value from .env, without the quotes around it

function Get-EnvValue([string[]]$Lines, [string]$Key) {

    $line = $Lines | Where-Object { $_ -like "$Key=*" } | Select-Object -First 1

    if (-not $line) { return "" }

    $value = $line.Substring($Key.Length + 1)

    if ($value -match "^'(.*)'$" -or $value -match '^"(.*)"$') { $value = $Matches[1] }

    $value

}

function Resolve-LocalBinary([string]$Path, [string]$Target) {

    $candidate = if ($Path) {

        $Path

    } else {

        Join-Path $RepoRoot "target\$Target\release\pcsc-rs"

    }

    $resolved = Resolve-Path -LiteralPath $candidate -ErrorAction SilentlyContinue

    if (-not $resolved -or -not (Test-Path -LiteralPath $resolved.Path -PathType Leaf)) {

        throw "Local client binary was not found: $candidate. Build it first or pass -Binary <file>."

    }

    $resolved.Path

}

function Stop-Remote {

    # Older toolbox ps does not support pidof or ps options. The loop records both

    # PIDs, and the ps fallback reads the stable USER PID ... NAME layout.

    $stopCommand = (@'

for pidfile in __DIR__/loop.pid __DIR__/client.pid; do

  if test -f $pidfile; then

    pid=$(cat $pidfile 2>/dev/null)

    case $pid in ''|*[!0-9]*) ;; *) kill -9 $pid 2>/dev/null ;; esac

    rm $pidfile 2>/dev/null || true

  fi

done

ps | while read user pid ppid vsize rss wchan pc state name; do

  case $name in pcsc-rs|*/pcsc-rs) kill -9 $pid 2>/dev/null ;; esac

done

true

'@ -replace "__DIR__", $Dir)

    Invoke-Remote $stopCommand | Out-Null

}

function Get-RemoteLog([int]$Count) {

    # Android 2.3 toolbox may not provide tail; fetch the file and trim it locally.

    $output = @(Invoke-Adb shell "cat $Dir/pcsc-rs.log 2>/dev/null") |

        ForEach-Object { "$_".TrimEnd("`r") }

    if ($LASTEXITCODE -ne 0) { throw "adb shell failed: cat $Dir/pcsc-rs.log" }

    if ($Count -gt 0) { $output | Select-Object -Last $Count } else { $output }

}

switch ($Command) {

    "install" {

        $abi = ((Invoke-Remote "getprop ro.product.cpu.abi") -join "").Trim()

        if (-not $abi) { $abi = ((Invoke-Remote "uname -m") -join "").Trim() }

        if ($abi -eq "aarch64" -or $abi -eq "arm64-v8a") {

            # Android 5.0+ arm64 only has the bionic build.

            if ($Libc -eq "musl") { throw "No musl build for arm64. Use -Libc bionic or auto." }

            $Targets = @("aarch64-linux-android")

        } elseif ($abi -eq "armv7l" -or $abi -eq "armeabi-v7a") {

            if ($Libc -eq "bionic") {

                # Needs Android 5.0 (API 21) or later.

                $Targets = @("armv7-linux-androideabi")

            } else {

                # Android 2.3.3+ 32-bit ARM uses a static musl build.

                $Targets = @("armv7-unknown-linux-musleabi")

            }

        } elseif ($abi -eq "armv6l" -or $abi -eq "armeabi" -or $abi -eq "arm") {

            if ($Libc -eq "bionic") { throw "No bionic build for armeabi. Use -Libc musl or auto." }

            $Targets = @("arm-unknown-linux-musleabi")

        } else {

            throw "Unsupported architecture: $abi (supported: aarch64, armeabi-v7a, armeabi)"

        }

        $Binary = Resolve-LocalBinary $Binary $Targets[0]

        "Installing local client ($($Targets[0])): $Binary"

        Invoke-Remote "mkdir -p $Dir" | Out-Null

            # Without -Hostname or PASS, keep the ones in the installed .env

            $oldEnv = @(Invoke-Adb shell "cat $Dir/.env 2>/dev/null") | ForEach-Object { "$_".TrimEnd("`r") }

            $pass = if ($env:PASS) { $env:PASS } else { Get-EnvValue $oldEnv "PASS" }

            if (-not $Hostname) { $Hostname = Get-EnvValue $oldEnv "HOSTNAME" }

            if (-not $pass) { throw "Set PASS first: `$env:PASS = `"<PASS>`"" }

            # Values go in single quotes, so spaces and symbols are read as they are; a quote cannot be

            if ("$pass$Hostname".Contains("'")) { throw "PASS and the name cannot contain '" }

            $envText = "PASS='$pass'`n"

            if ($Hostname) { $envText += "HOSTNAME='$Hostname'`n" }

            # Keep the other settings (PCSC_URI, DEV_MODE, ...)

            foreach ($line in $oldEnv) {

                if ($line -and $line -notmatch "^(PASS|PCSC_UPDATED|HOSTNAME)=") { $envText += "$line`n" }

            }

            Stop-Remote

            Invoke-Adb push $Binary "$Dir/pcsc-rs" | Out-Null

            if ($LASTEXITCODE -ne 0) { throw "adb push failed" }

            Send-File $envText "$Dir/.env"

            # Restart the client 5 seconds after it exits (crash). .env is read from the working directory.

            Invoke-Remote "rm -f $Dir/loop.sh $Dir/loop.pid $Dir/client.pid" | Out-Null

$loopScript = @'

#!/system/bin/sh

cd /data/local/tmp/pcsc-rs || exit 1

echo $$ > loop.pid

# Use the hostname configured in .env, not adb shell's HOSTNAME.

unset HOSTNAME

while true; do

  ./pcsc-rs &

  client_pid=$!

  echo $client_pid > client.pid

  wait $client_pid

  rm -f client.pid

  sleep 5

done

'@

Send-File $loopScript "$Dir/loop.sh"

Invoke-Remote "chmod 600 $Dir/.env; chmod 755 $Dir/pcsc-rs $Dir/loop.sh" | Out-Null

# Clear the old log so that the connection check uses this run's output.

Invoke-Remote "cd $Dir && : > pcsc-rs.log" | Out-Null

# Detach all standard streams; old Android toolbox does not provide setsid.

Invoke-Remote "cd $Dir && (./loop.sh > $Dir/pcsc-rs.log 2>&1 < /dev/null &)" | Out-Null

$logLines = @(Get-RemoteLog 8)

if ($logLines.Count -gt 0) {

    $logLines

} else {

    "Client started; no log output yet. Run 'scripts/android-adb.ps1 log' to check again."

}

    }

    "stop" {

        Stop-Remote

        "Stopped"

    }

    "status" {

        $ps = @(Invoke-Remote "ps | while read user pid ppid vsize rss wchan pc state name; do case `$name in pcsc-rs|*/pcsc-rs) echo `$user `$pid `$ppid `$vsize `$rss `$wchan `$pc `$state `$name ;; esac; done")

        if ($ps.Count -gt 0) { $ps } else { "Not running" }

    }

    "log" {

        Get-RemoteLog $Lines

    }

}
