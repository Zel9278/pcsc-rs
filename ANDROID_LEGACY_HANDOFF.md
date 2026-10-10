# Android 2.3 対応 引き継ぎメモ

## 目的と依頼内容

- 現行Androidアプリを古い端末でも使えるようにする。
- 対応OSは Android 2.3.3（API 10）〜Android 6.x（API 23）。
- 配布形式はできれば APK。
- 既存機能は可能な限り維持する。
- ARMv6 / ARMv7 の両方を用意する。
- 想定実機は NEC MEDIAS WP N-06C。端末仕様上は Android 2.3.3、Qualcomm MSM8255 / Scorpion ARMv7。

## 作業ツリーの現状

作業ディレクトリは `C:\Users\SHINA\Documents\GitHub\pcsc-rs2`。下記の変更は未コミットで、現状まだAPK成果物は作成できていない。

変更ファイル:

- `.github/workflows/build.yml`
- `Cargo.toml`
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
- ソースの目視レビューで常駐起動ループが `.env` をシェルsourceしようとしていた箇所を除去。アプリ本体は `src/main.rs` の `dotenvy::from_path_iter` で `.env` を読むため、シェルsourceは不要。
- 設定ファイル読み込みと、Android側のlaunch/boot処理を目視で確認。

## 未実施・未確認のこと（重要）

1. **APKをまだビルドしていない。** この環境では必要なJDK 21 / Android NDKなどを用意できず、Gradleビルドは未確認。root端末向けAPI 10 APKを実機にインストールできるかも未確認。N-06CはAPKではなくPC側adbを使う。
2. **aarch64/ARMv6/ARMv7のRustビルドは未確認。** とくにrustlsが依存するringのARMv6サポート/アセンブリ選択がリスク。CIでAPI 21のaarch64と両32bitターゲットを実際にビルドすること。
3. **N-06Cでの起動・TLS・通信・DNS・adb shell動作は未確認。** `adb shell` の常駐化、`pidof` / `tail` 等の古いtoolbox実装差を実機で確認すること。
4. **カーネル互換性リスク。** N-06CのLinux 2.6.35カーネルはmuslがPOSIX準拠を想定する2.6.39以降を下回る。adb shellで実行できる可能性はあるが、実機検証なしに動作保証しない。Android 5〜6のaarch64はAPI 21ビルドを使う。
5. **旧APKの更新。** 旧APKは自動更新を行わない。新しいクライアントは新しいAPKを配布して更新する。
6. **ARMv6を含む両方のABIが本当にAPKに入ること。** `checkLegacyClients` は両assetsを必須とする。CIのartifact download先と `debug` / `release` の配置を確認する。
7. **署名・配布。** リリースAPKは既存keystore secretsの設定が必要。PRのdebug署名APKは正式配布用ではない。
8. **GitHub Actions YAMLの実行確認がない。** workflow expressions、`cross` の実行環境、artifactディレクトリの実体、Android Gradle Plugin 9.4.1でAPI 10向けAPKを生成できるか確認する。

## 次に行う作業（推奨順）

1. `ANDROID_LEGACY_HANDOFF.md` と現在の差分を読んで作業を再開する。
2. CIを回すか同等環境で以下を順番に実行し、最初の失敗を解消する。
   - `cargo build --locked --bins --release --target aarch64-linux-android --no-default-features --features android-legacy`
   - `cross build --locked --bins --release --target arm-unknown-linux-musleabi --no-default-features --features android-legacy`
   - `cross build --locked --bins --release --target armv7-unknown-linux-musleabi --no-default-features --features android-legacy`
   - 2成果物を `android/legacy/src/main/assets/bin/armeabi/pcsc-rs` と `armeabi-v7a/pcsc-rs` に配置。
   - Java 21 + Android SDK 37で `./gradlew :legacy:assembleRelease`。
3. APKをzipとして確認し、ManifestのminSdkが10、両ARM payloadが存在することを検証。
4. N-06Cなどの実機でADB許可・バイナリ配置・起動/停止・接続・TLS・DNSを確認。再起動後はADBから再度`install`する。
5. ringがARMv6ビルドを拒否する場合は、ARMv6 TLS依存を別方式にするか、機能差を明記してARMv6側の提供仕様を再検討。確認前にARMv6対応済みと断定しない。
6. `scripts/android-adb.sh` / `.ps1` のログ/停止処理をAndroid 2.3〜6.xのtoolbox環境で確認し、プロセス残留や二重起動がないか見る。

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
