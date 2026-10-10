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
    [int]$CompileSdk = 0
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
        return $explicitRoots[0]
    }

    $sdkRoots = @($env:ANDROID_SDK_ROOT, $env:ANDROID_HOME) |
        Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Container) }
    foreach ($sdkRoot in $sdkRoots) {
        $ndkDirectory = Join-Path $sdkRoot "ndk"
        if (Test-Path -LiteralPath $ndkDirectory -PathType Container) {
            $latest = Get-ChildItem -LiteralPath $ndkDirectory -Directory |
                Sort-Object Name | Select-Object -Last 1
            if ($latest) {
                return $latest.FullName
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
    $ndkRoot = Get-NdkRoot
    if (-not $ndkRoot) {
        throw "Android NDK was not found. Set ANDROID_NDK_ROOT or ANDROID_SDK_ROOT and install an NDK."
    }

    $ndkBin = Join-Path $ndkRoot "toolchains\llvm\prebuilt\windows-x86_64\bin"
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
    Require-Command "cross" "Install cross with: cargo install cross --locked"
    Require-Command "docker" "Install Docker Desktop and make docker.exe available on PATH."
    Invoke-External "docker" @("info")

    Push-Location $RepoRoot
    try {
        foreach ($target in @("arm-unknown-linux-musleabi", "armv7-unknown-linux-musleabi")) {
            Invoke-External "cross" @(
                "build", "--locked", "--release", "--target", $target,
                "--no-default-features", "--features", "android-legacy"
            )
        }
    } finally {
        Pop-Location
    }
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
