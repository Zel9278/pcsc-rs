#!/usr/bin/env bash
set -euo pipefail

# Build Android artifacts without GitHub Actions.
#
# Usage:
#   scripts/build-android-local.sh android64
#   scripts/build-android-local.sh apk
#
# The Android 64-bit build uses the locally installed NDK. The 32-bit
# Gingerbread builds use cross and therefore require a working Docker daemon.

repo_root="$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
build_kind="${1:-android64}"
compile_sdk="${ANDROID_COMPILE_SDK:-37}"

case "${HOME:-}" in
  /*) ;;
  *) HOME="$(getent passwd "$(id -un)" 2>/dev/null | cut -d: -f6 || true)"; export HOME ;;
esac
if [ -d "$HOME/.cargo-lavuntu/bin" ]; then
  export CARGO_HOME="$HOME/.cargo-lavuntu"
  export RUSTUP_HOME="$HOME/.rustup-lavuntu"
fi
export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$HOME/.cargo/bin:$PATH"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-1}"

case "$build_kind" in
  android64)
    targets="aarch64-linux-android"
    ;;
  arm32)
    targets="arm-unknown-linux-musleabi armv7-unknown-linux-musleabi"
    ;;
  all | apk)
    targets="aarch64-linux-android arm-unknown-linux-musleabi armv7-unknown-linux-musleabi"
    ;;
  *)
    echo "usage: $0 {android64|arm32|all|apk}" >&2
    exit 2
    ;;
esac

command -v cargo >/dev/null 2>&1 || {
  echo "cargo が見つかりません。rustupをインストールしてから再実行してください。" >&2
  exit 1
}
command -v rustup >/dev/null 2>&1 || {
  echo "rustup が見つかりません。rustupをインストールしてから再実行してください。" >&2
  exit 1
}

cd "$repo_root"
rustup target add $targets

ndk_root="${ANDROID_NDK_ROOT:-${ANDROID_NDK_HOME:-}}"
if [ -z "$ndk_root" ] && [ -n "${ANDROID_SDK_ROOT:-}" ]; then
  ndk_root="$(find "$ANDROID_SDK_ROOT/ndk" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -V | tail -n 1 || true)"
fi
if [ -z "$ndk_root" ] && [ -n "${ANDROID_HOME:-}" ]; then
  ndk_root="$(find "$ANDROID_HOME/ndk" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort -V | tail -n 1 || true)"
fi

build_android64() {
  if [ -z "$ndk_root" ] || [ ! -d "$ndk_root/toolchains/llvm/prebuilt/linux-x86_64/bin" ]; then
    cat >&2 <<'EOF'
Android NDKが見つかりません。
ANDROID_NDK_ROOTまたはANDROID_SDK_ROOTを設定し、NDKをインストールしてください。
例: sdkmanager "platform-tools" "platforms;android-36" "ndk;27.2.12479018"
EOF
    exit 1
  fi

  local ndk_bin="$ndk_root/toolchains/llvm/prebuilt/linux-x86_64/bin"
  local linker="$ndk_bin/aarch64-linux-android21-clang"
  [ -x "$linker" ] || {
    echo "NDK API 21 linkerが見つかりません: $linker" >&2
    exit 1
  }

  CC_aarch64_linux_android="$linker" \
  AR_aarch64_linux_android="$ndk_bin/llvm-ar" \
  CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$linker" \
  cargo build --locked --release --target aarch64-linux-android \
    --no-default-features --features android-legacy
}

build_arm32() {
  command -v cross >/dev/null 2>&1 || {
    echo "arm32ビルドにはcrossが必要です: cargo install cross --locked" >&2
    exit 1
  }
  docker info >/dev/null 2>&1 || {
    echo "arm32ビルドにはDocker daemonが必要です。Docker Desktopを起動してください。" >&2
    exit 1
  }
  for target in arm-unknown-linux-musleabi armv7-unknown-linux-musleabi; do
    cross build --locked --release --target "$target" \
      --no-default-features --features android-legacy
  done
}

prepare_apk_assets() {
  mkdir -p android/app/src/main/jniLibs/arm64-v8a
  cp target/aarch64-linux-android/release/pcsc-rs \
    android/app/src/main/jniLibs/arm64-v8a/libpcsc.so
  mkdir -p android/legacy/src/main/assets/bin/armeabi \
    android/legacy/src/main/assets/bin/armeabi-v7a
  cp target/arm-unknown-linux-musleabi/release/pcsc-rs \
    android/legacy/src/main/assets/bin/armeabi/pcsc-rs
  cp target/armv7-unknown-linux-musleabi/release/pcsc-rs \
    android/legacy/src/main/assets/bin/armeabi-v7a/pcsc-rs
}

case "$build_kind" in
  android64) build_android64 ;;
  arm32) build_arm32 ;;
  all)
    build_android64
    build_arm32
    ;;
  apk)
    build_android64
    build_arm32
    prepare_apk_assets
    (
      cd android
      ./gradlew --no-daemon --max-workers=2 :app:assembleRelease :legacy:assembleRelease \
        -Pandroid.compileSdk="$compile_sdk"
    )
    ;;
esac

echo "ローカルビルド完了"
