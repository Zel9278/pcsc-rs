# pcsc-rs Android app

The aarch64-linux-android build of pcsc-rs in an APK, started with root or Shizuku.
It runs the client the same way as `scripts/android-adb.sh`: copied to `/data/local/tmp/pcsc-rs`
and kept running by a detached loop, so it keeps running after the app is closed.

## Build

The client binary goes into the APK as `libpcsc.so`, so Android extracts it next to the app,
where root or the shell user can copy it from:

```sh
# The Android build of the client (the local script uses the NDK, API 21)
cargo build --release --target aarch64-linux-android --no-default-features --features android-legacy
mkdir -p android/app/src/main/jniLibs/arm64-v8a
cp target/aarch64-linux-android/release/pcsc-rs android/app/src/main/jniLibs/arm64-v8a/libpcsc.so

cd android
./gradlew assembleRelease   # app/build/outputs/apk/release/app-release.apk
```

Needs JDK 17 or later and the Android SDK (`ANDROID_HOME`, or `sdk.dir` in `local.properties`).

For a local build from WSL, install Rust and the Android SDK command-line tools,
then run `scripts/build-android-local.sh android64` from the repository root.
The script detects the newest installed NDK and configures the API 21 linker.
Use `scripts/build-android-local.sh arm32` for the ARMv6/ARMv7 static builds;
that mode requires `cross` and a running Docker daemon. In WSL, enable Docker
Desktop integration for the selected WSL distribution; calling `docker.exe`
without WSL integration does not provide the bind mounts required by `cross`.
To build both APKs locally, run `scripts/build-android-local.sh apk`. This also copies
the binaries into the APK modules and runs Gradle with at most two workers. The current
Compose dependencies require Android SDK Platform 37; install it if it is not present.
The resulting files are `android/app/build/outputs/apk/release/app-release.apk` and
`android/legacy/build/outputs/apk/release/legacy-release-unsigned.apk`.

On a Windows host, run the PowerShell equivalent from the repository root:
`.\scripts\build-android-local.ps1 android64`. Use `arm32` for the ARMv6/ARMv7
builds or `apk` to build both APKs. The PowerShell script uses the Windows NDK
toolchain directly for ARM64 and delegates ARMv6/ARMv7 builds to WSL. The latter
requires Rust, `cross`, and Docker Desktop WSL integration in the selected WSL distribution.

Without the signing variables below the release APK is signed with the debug key.

| Variable | |
|---|---|
| `ANDROID_KEYSTORE` | Path to the keystore |
| `ANDROID_KEYSTORE_PASSWORD` | |
| `ANDROID_KEY_ALIAS` | |
| `ANDROID_KEY_PASSWORD` | |

The CI takes them from the secrets `ANDROID_KEYSTORE_BASE64` (the keystore, base64),
`ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS` and `ANDROID_KEY_PASSWORD`. Releases attach
the APK only when the key is there.

The app's version is the client's (`Cargo.toml`).

## Older Android (Gingerbread)

The separate `legacy` APK targets Android 2.3.3 (API 10) and bundles 32-bit ARMv6 and ARMv7
static clients. It requires root. On an unrooted phone such as the N-06C, use the PC-side
`scripts/android-adb.sh` or `scripts/android-adb.ps1` flow documented in the main README
instead; it runs as the adb shell user and does not require an APK.

The PC-side ADB flow covers Android 2.3.3 through 6.x (API 10 through 23): it uses static
musl ARMv6/ARMv7 clients on 32-bit devices and the API 21 `aarch64-linux-android` client on
64-bit devices. The legacy client does not perform self-updates; run `install` from the PC to
install a newer client.
