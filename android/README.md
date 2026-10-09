# pcsc-rs Android app

The aarch64-linux-android build of pcsc-rs in an APK, started with root or Shizuku.
It runs the client the same way as `scripts/android-adb.sh`: copied to `/data/local/tmp/pcsc-rs`
and kept running by a detached loop, so it keeps running after the app is closed.

## Build

The client binary goes into the APK as `libpcsc.so`, so Android extracts it next to the app,
where root or the shell user can copy it from:

```sh
# The Android build of the client (the CI builds it with the NDK, API 24)
cargo build --release --target aarch64-linux-android
mkdir -p android/app/src/main/jniLibs/arm64-v8a
cp target/aarch64-linux-android/release/pcsc-rs android/app/src/main/jniLibs/arm64-v8a/libpcsc.so

cd android
./gradlew assembleRelease   # app/build/outputs/apk/release/app-release.apk
```

Needs JDK 17 or later and the Android SDK (`ANDROID_HOME`, or `sdk.dir` in `local.properties`).

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
