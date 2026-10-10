# Build Android artifacts without GitHub Actions from a Windows host.
#
# Usage:
#   .\scripts\build-android-local.ps1 android64
#   .\scripts\build-android-local.ps1 arm32
#   .\scripts\build-android-local.ps1 apk
#
# android64 uses the Windows NDK directly. arm32 uses cross and Docker Desktop.
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [ValidateSet("android64", "arm32", "all", "apk")]
    [string]$BuildKind = "android64",
    [int]$CompileSdk = 0,
    [string]$WslDistribution = "Lavuntu"
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

function Require-Command([string]$Name, [string]$Hint) {
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "$Name was not found. $Hint"
    }
}

function Invoke-External([string]$FilePath, [string[]]$Arguments) {
    & $FilePath @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "$FilePath failed with exit code $LASTEXITCODE"
    }
}

function Get-NdkRoot {
    $explicitRoots = @($env:ANDROID_NDK_ROOT, $env:ANDROID_NDK_HOME) |
        Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Container) }
    if ($explicitRoots.Count -gt 0) {
        return [string]($explicitRoots | Select-Object -First 1)
    }

    $sdkRoots = @($env:ANDROID_SDK_ROOT, $env:ANDROID_HOME) |
        Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Container) }
    foreach ($sdkRoot in $sdkRoots) {
        $ndkDirectory = Join-Path $sdkRoot "ndk"
        if (Test-Path -LiteralPath $ndkDirectory -PathType Container) {
            $latest = Get-ChildItem -LiteralPath $ndkDirectory -Directory |
                Sort-Object Name | Select-Object -Last 1
            if ($latest) {
                return [string]$latest.FullName
            }
        }
    }

    return $null
}

function Add-AndroidTarget {
    Require-Command "rustup" "Install Rust with rustup and make it available on PATH."
    Invoke-External "rustup" @("target", "add", "aarch64-linux-android")
}

function Build-Android64 {
    $ndkRoot = [string](Get-NdkRoot)
    if (-not $ndkRoot) {
        throw "Android NDK was not found. Set ANDROID_NDK_ROOT or ANDROID_SDK_ROOT and install an NDK."
    }

    $ndkBin = Join-Path -Path $ndkRoot.Trim() -ChildPath "toolchains\llvm\prebuilt\windows-x86_64\bin"
    if (-not (Test-Path -LiteralPath $ndkBin -PathType Container)) {
        throw "Windows NDK toolchain was not found: $ndkBin"
    }

    $linker = Join-Path $ndkBin "aarch64-linux-android21-clang.cmd"
    if (-not (Test-Path -LiteralPath $linker -PathType Leaf)) {
        $linker = Join-Path $ndkBin "aarch64-linux-android21-clang.exe"
    }
    if (-not (Test-Path -LiteralPath $linker -PathType Leaf)) {
        throw "Android API 21 linker was not found in $ndkBin"
    }

    $env:CC_aarch64_linux_android = $linker
    $env:AR_aarch64_linux_android = Join-Path $ndkBin "llvm-ar.exe"
    $env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = $linker

    Push-Location $RepoRoot
    try {
        Invoke-External "cargo" @(
            "build", "--locked", "--release", "--target", "aarch64-linux-android",
            "--no-default-features", "--features", "android-legacy"
        )
    } finally {
        Pop-Location
    }
}

function Build-Arm32 {
    Require-Command "wsl.exe" "Install WSL and a Linux distribution with Rust, cross, and Docker Desktop WSL integration."
    $wslInput = $RepoRoot -replace "\\", "/"
    $wslRoot = ((& wsl.exe -d $WslDistribution -- wslpath -a $wslInput 2>$null) -join "").Trim()
    if ($LASTEXITCODE -ne 0 -or -not $wslRoot) {
        throw "Could not resolve the repository path in WSL distribution '$WslDistribution'."
    }

    $wslCommand = 'export HOME=$(getent passwd "$(id -un)" | cut -d: -f6); ' +
        'export CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}"; ' +
        'export RUSTUP_HOME="${RUSTUP_HOME:-$HOME/.rustup}"; ' +
        'if [ -d "$HOME/.cargo-lavuntu/bin" ]; then ' +
        'export CARGO_HOME="$HOME/.cargo-lavuntu"; ' +
        'export RUSTUP_HOME="$HOME/.rustup-lavuntu"; fi; ' +
        'export PATH="$CARGO_HOME/bin:$HOME/.cargo/bin:$PATH"; ' +
        'export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"; ' +
        'command -v cargo >/dev/null 2>&1 || { echo "cargo is missing in WSL" >&2; exit 127; }; ' +
        'command -v cross >/dev/null 2>&1 || { echo "cross is missing in WSL" >&2; exit 127; }; ' +
        'command -v docker >/dev/null 2>&1 || { echo "Docker is missing in WSL" >&2; exit 127; }; ' +
        'docker info >/dev/null 2>&1 || { echo "Docker Desktop WSL integration is unavailable" >&2; exit 125; }; ' +
        'cd "' + $wslRoot + '"; bash scripts/build-android-local.sh arm32'
    Invoke-External "wsl.exe" @("-d", $WslDistribution, "--", "bash", "-lc", $wslCommand)
}

function Prepare-ApkAssets {
    $arm64Directory = Join-Path $RepoRoot "android\app\src\main\jniLibs\arm64-v8a"
    New-Item -ItemType Directory -Force -Path $arm64Directory | Out-Null
    Copy-Item -Force `
        (Join-Path $RepoRoot "target\aarch64-linux-android\release\pcsc-rs") `
        (Join-Path $arm64Directory "libpcsc.so")

    $armeabiDirectory = Join-Path $RepoRoot "android\legacy\src\main\assets\bin\armeabi"
    $armeabiV7Directory = Join-Path $RepoRoot "android\legacy\src\main\assets\bin\armeabi-v7a"
    New-Item -ItemType Directory -Force -Path $armeabiDirectory, $armeabiV7Directory | Out-Null
    Copy-Item -Force `
        (Join-Path $RepoRoot "target\arm-unknown-linux-musleabi\release\pcsc-rs") `
        (Join-Path $armeabiDirectory "pcsc-rs")
    Copy-Item -Force `
        (Join-Path $RepoRoot "target\armv7-unknown-linux-musleabi\release\pcsc-rs") `
        (Join-Path $armeabiV7Directory "pcsc-rs")
}

Require-Command "cargo" "Install Rust with rustup and make it available on PATH."
if ($CompileSdk -eq 0) {
    $CompileSdk = if ($env:ANDROID_COMPILE_SDK) { [int]$env:ANDROID_COMPILE_SDK } else { 37 }
}

switch ($BuildKind) {
    "android64" {
        Add-AndroidTarget
        Build-Android64
    }
    "arm32" {
        Build-Arm32
    }
    "all" {
        Add-AndroidTarget
        Build-Android64
        Build-Arm32
    }
    "apk" {
        Add-AndroidTarget
        Build-Android64
        Build-Arm32
        Prepare-ApkAssets

        $gradle = Join-Path $RepoRoot "android\gradlew.bat"
        if (-not (Test-Path -LiteralPath $gradle -PathType Leaf)) {
            throw "Gradle wrapper was not found: $gradle"
        }
        Push-Location (Join-Path $RepoRoot "android")
        try {
            Invoke-External $gradle @(
                "--no-daemon", "--max-workers=2",
                ":app:assembleRelease", ":legacy:assembleRelease",
                "--project-prop", "android.compileSdk=$CompileSdk"
            )
        } finally {
            Pop-Location
        }
    }
}

Write-Output "Local Android build completed."
