# Android 2.3 対応 引き継ぎメモ

## 目的と依頼内容

- 現行Androidアプリを古い端末でも使えるようにする。
- 対応OSは Android 2.3.3（API 10）〜Android 6.x（API 23）。
- 配布形式はできれば APK。
- 既存機能は可能な限り維持する。
- ARMv6 / ARMv7 の両方を用意する。
- 想定実機は NEC MEDIAS WP N-06C。端末仕様上は Android 2.3.3、Qualcomm MSM8255 / Scorpion ARMv7。

## 作業ツリーの現状

作業ディレクトリは `C:\Users\SHINA\Documents\GitHub\pcsc-rs2`。下記の変更は未コミット。
ローカル環境でARM64・ARMv6・ARMv7のビルドと、現行・legacy両APKのreleaseビルドまで確認済み。

変更ファイル:

- `.github/workflows/build.yml`
- `scripts/build-android-local.sh`
- `scripts/build-android-local.ps1`
- `Cargo.toml`
- `android/.gitignore`
- `android/README.md`
- `android/settings.gradle.kts`
- `android/legacy/` 以下（API 10用の新規Androidアプリ）
- `src/client.rs`
- `src/lib.rs`
- `src/legacy_dns.rs`（新規）

## 実施済みの実装

### Androidアプリ

- `android/legacy/build.gradle.kts` に新しい `:legacy` APKモジュールを追加。
  - `minSdk = 10`、`targetSdk = 23`、`compileSdk = 37`。
  - Javaで実装し、AndroidX/Composeを使わずGingerbreadのAPIに寄せた。
- `android/settings.gradle.kts` で `:legacy` をinclude。
- `android/legacy/src/main/AndroidManifest.xml` に起動Activity、起動時BroadcastReceiver、ネットワーク権限、API 10を設定。
- `MainActivity.java` にパスワード、ホスト名、接続先URI、自動起動、状態/ログ、起動/再起動/停止の画面を実装。
- `Settings.java` に設定のSharedPreferences保存とシェル値の簡易バリデーションを実装。
- `BootReceiver.java` に端末再起動時の自動起動を実装。
- `RootClient.java` でroot端末向けに実行ファイルを `/data/local/tmp/pcsc-rs` に配置し、shell UID 2000で常駐ループを起動する構成を追加。rootを取れないN-06CではAPKを使わず、PC側のadbスクリプトを使う。
- Gingerbreadでは利用できないShizukuと無線デバッグペアリングは対象外。旧APKはroot前提で、N-06CはPCからadb shellで操作する。

### Rustクライアント

- `Cargo.toml` に `android-legacy` featureを追加。TLSはrustls + bundled webpki rootsを使い、旧APKでは自動更新を無効化する。
- `src/client.rs` にARM Linux musl向けDNSフォールバックを追加。
- `src/legacy_dns.rs` を新規作成。`/system/bin/getprop net.dns1/net.dns2` からIPv4 DNSサーバーを取り、Aレコード照会を行う。
- `src/lib.rs` で当該feature時はAndroid TLS証明書ディレクトリ設定を実行しないよう条件を調整。

### CI / 配布

- `.github/workflows/build.yml` にAPI 21の `aarch64-linux-android`、`arm-unknown-linux-musleabi`、`armv7-unknown-linux-musleabi` のビルド行列を追加。
- 対象は `cross` を使ってビルドする設定。
- Android APK jobで両クライアントをassetsへ配置し、現行 `:app` と旧 `:legacy` のAPKをビルド・成果物化する設定。
- リリース時は通常Android APKに加え `pcsc-rs-<tag>-android-legacy.apk` も添付する。
- `android/README.md` とルートの `README.md` に、N-06Cなどrootなし端末ではPCからadbを使う説明を追記。

## 実施した確認

- `cargo fmt` を実行。
- `git diff --check` を実行し、空白エラーなし。
- `cargo check --locked --no-default-features --features android-legacy` 成功。
- `cargo test --locked --no-default-features --features android-legacy` 成功（24件）。
- `scripts/build-android-local.sh` のBash構文チェック成功。
- ARM64 Android、ARMv6 musl、ARMv7 muslのreleaseビルド成功。
- 現行APKとlegacy APKのreleaseビルド成功。
- legacy APKは`minSdkVersion=10`、`targetSdkVersion=23`で、ARMv6/ARMv7 payloadを含むことを確認。
- ソースの目視レビューで常駐起動ループが `.env` をシェルsourceしようとしていた箇所を除去。アプリ本体は `src/main.rs` の `dotenvy::from_path_iter` で `.env` を読むため、シェルsourceは不要。
- 設定ファイル読み込みと、Android側のlaunch/boot処理を目視で確認。

## 未実施・未確認のこと（重要）

1. **N-06Cでの起動・TLS・通信・DNS・adb shell動作は未確認。** `adb shell` の常駐化、`pidof` / `tail` 等の古いtoolbox実装差を実機で確認すること。
2. **カーネル互換性リスク。** N-06CのLinux 2.6.35カーネルはmuslがPOSIX準拠を想定する2.6.39以降を下回る。adb shellで実行できる可能性はあるが、実機検証なしに動作保証しない。Android 5〜6のaarch64はAPI 21ビルドを使う。
3. **旧APKの更新。** 旧APKは自動更新を行わない。新しいクライアントは新しいAPKを配布して更新する。
4. **署名・配布。** リリースAPKは既存keystore secretsの設定が必要。ローカルのdebug署名APKは正式配布用ではない。
5. **GitHub Actions YAMLの実行確認がない。** workflow expressions、`cross` の実行環境、artifactディレクトリの実体はGitHub Actions上では未確認。

## 次に行う作業（推奨順）

1. `ANDROID_LEGACY_HANDOFF.md` と現在の差分を読んで作業を再開する。
2. ローカルで以下を順番に実行し、最初の失敗を解消する。
   - `scripts/build-android-local.sh android64`
   - `scripts/build-android-local.sh arm32`
   - WindowsではAndroid Studio付属JDK 21とAndroid SDK Platform 37でAPKをビルドする。
     `android\gradlew.bat --no-daemon --max-workers=2 :app:assembleRelease :legacy:assembleRelease`
     （`android/legacy/src/main/assets`と`android/app/src/main/jniLibs`へ先にバイナリを配置する。）
   - WindowsホストではPowerShell版も利用できる。
     `.\scripts\build-android-local.ps1 apk`
3. WSLのDocker Desktop連携が有効なら、以下の個別コマンドでも確認できる。
   - `cargo build --locked --bins --release --target aarch64-linux-android --no-default-features --features android-legacy`
   - `cross build --locked --bins --release --target arm-unknown-linux-musleabi --no-default-features --features android-legacy`
   - `cross build --locked --bins --release --target armv7-unknown-linux-musleabi --no-default-features --features android-legacy`
   - 2成果物を `android/legacy/src/main/assets/bin/armeabi/pcsc-rs` と `armeabi-v7a/pcsc-rs` に配置。
   - Java 21 + Android SDK 37で `./gradlew :legacy:assembleRelease`。
4. APKをzipとして確認し、ManifestのminSdkが10、両ARM payloadが存在することを検証。
5. N-06Cなどの実機でADB許可・バイナリ配置・起動/停止・接続・TLS・DNSを確認。再起動後はADBから再度`install`する。
6. ringがARMv6ビルドを拒否する場合は、ARMv6 TLS依存を別方式にするか、機能差を明記してARMv6側の提供仕様を再検討。確認前にARMv6対応済みと断定しない。
7. `scripts/android-adb.sh` / `.ps1` のログ/停止処理をAndroid 2.3〜6.xのtoolbox環境で確認し、プロセス残留や二重起動がないか見る。

## 参照情報

- N-06C仕様: https://phonedb.net/index.php?c=nec_medias_wp_n-06c&d=detailed_specs&id=2871&m=Device
- Qualcomm Scorpion ARMv7-A資料: https://www.qualcomm.com/content/dam/qcomm-martech/dm-assets/documents/M29__Qualcomm_PDFa.pdf
- Rust Android NDK更新（NDK r25 / API 19 baseline）: https://blog.rust-lang.org/2023/01/09/android-ndk-update-r25/
- Android NDK互換性資料: https://github.com/android/ndk/wiki/Compatibility
- musl対応プラットフォーム / kernel要件: https://wiki.musl-libc.org/supported-platforms
- crossプロジェクト: https://github.com/cross-rs/cross
- ring ARMv6ビルド課題の報告（一次資料・古いissueのため現行バージョンで再確認が必要）: https://github.com/briansmith/ring/issues/516

## 引き継ぎ時の注意

作業ツリーには既存の未コミット変更として上記実装が残っている。最初に `git status --short` と `git diff` を確認し、変更を破棄・上書きしないこと。テストスイートはまだ実行していない。現時点の確認はformatとdiff whitespaceの範囲に限られる。
