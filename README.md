# PC Status Client - Rust

PC の状態を取得し、[PC Status](https://pc-stats.eov2.com/)に送信、表示するツールです。

![Preview - Overview](docs/images/overview.png)
| ![Preview - Focus to DMS-Arch](docs/images/focus-dms.png) | ![Preview - Focus to ZPC-Fedora](docs/images/focus-zpc.png) |
|---|---|

## 注意

**ツールの性質上、以下の内容が他者に誰でも見られる状態で送信されるため、少しでも不快感を感じる場合は使用しないでください。**\
個人に繋がるような情報は**ホスト名を除き**送信される事はありませんが、必要に応じて PC のホスト名を変更するか、或いは `.env` 内に以下の Key を追加してください。

```env
HOSTNAME=ホスト名として表示させたい文字列
```

## 送信、表示内容

1. PC のホスト名 (e.g. `assault-e5dmts`)
   - ホスト名に個人情報などが含まれている場合は上記[注意](#注意)を参考にホスト名を変更してください。
   - 64 文字まで受け付けますが、32 文字目以降は ... で隠されます。\
     マウスオーバーで全て表示されます。
2. OS のバージョン (e.g. `Windows 10 Home(Windows_NT win32 x64 10.0.19044)` or `Windows 10 (19045)`)
3. CPU 名、CPU 使用率 (全体, コアごと) (e.g. `AMD Ryzen 5 3500 6-Core Processor`)
4. 物理メモリ使用量、スワップメモリ使用量
5. マウントされているストレージ使用量 (実行されている root を参照)
6. GPU 使用率、GPU メモリ使用量 (NVIDIA GPU のみ、複数枚対応)
7. 連続起動時間
8. Load Average (Linux のみ)

## 対応環境

最近の Windows (Server 含む), macOS, Linux であれば動作すると思います。\
もし動作しない場合は [Issues](https://github.com/Zel9278/pcsc-rs/issues) から報告をお願いします。

## 使い方

1. [リリースページ](https://github.com/Zel9278/pcsc-rs/releases)から使用する環境に合った最新のリリースをダウンロードしてください。
2. 適当なフォルダに保存し、同じフォルダに `.env` ファイルを作成して以下の Key を追加してください。

```env
PASS=npU7pmkkYfuUdKfqzm2BtDfBPEe4pizrXyPVj8Fby3KaUtehNu3ToDtM8uEdGBr3AS9LRUkZixtZxuKTvsL2e4BVrfzWWG7RqqVThLWsVLHLaJJ8ekeGuHtLBkfZpBtv
```

3. ダウンロードしたリリースを実行してください。\
   macOS, Linux 環境では予め `$ chmod +x <ファイル名>` で実行権限を付与する必要があります。
4. [PC Status](https://pc-stats.eov2.com/)にアクセスし、自分の PC が表示されていれば完了です。

必要に応じて `pcsc-rs.exe` のショートカットを `shell:startup` に追加すれば、PC と同時に起動するようになります。

Linuxの場合、`sudo install -D --no-target-directory pcsc-rs-* /usr/local/bin/pcsc-rs`を実行し、Systemdに登録します。\
`sudo --preserve-env=EDITOR systemctl edit --force --full pcsc-rs.service`
```
[Unit]
Description=PCStatus Client
After=network-online.target

[Service]
Environment="PASS=npU7pmkkYfuUdKfqzm2BtDfBPEe4pizrXyPVj8Fby3KaUtehNu3ToDtM8uEdGBr3AS9LRUkZixtZxuKTvsL2e4BVrfzWWG7RqqVThLWsVLHLaJJ8ekeGuHtLBkfZpBtv"
Environment="PCSC_UPDATED=terminate"
ExecStart=/usr/local/bin/pcsc-rs
Restart=always

[Install]
WantedBy=network-online.target
```
```sh
sudo systemctl enable --now pcsc-rs
```

## その他の設定

- `PCSC_UPDATED`

  更新処理後の動作の設定。新しいリリースの確認は起動時と、起動中は6時間ごとに行います。

  | 値          | 説明                      |
  | ----------- | ------------------------- |
  | `none`      | なにもしない (デフォルト) |
  | `terminate` | 終了する                  |
  | `restart`   | 再起動する                |

- `PCSC_URI`

  接続先のサーバー (デフォルト: `wss://pcss.eov2.com/server`)\
  v2 から WebSocket で接続します。v1 と同じ `https://pcss.eov2.com` のような形でも、`wss://…/server` に読み替えて接続します。

- `DEV_MODE`

  `true` にすると同じホスト名で複数台つなげられます。PC Status には `[DEV] ホスト名_番号` と表示されます。

- `HOSTNAME`

  PC Status に表示するホスト名 (デフォルト: PC のホスト名)
