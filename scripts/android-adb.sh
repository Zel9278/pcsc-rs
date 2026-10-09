#!/bin/sh
# Android で pcsc-rs を動かし続ける（PC から adb で入れる）。root は要らない。
#
#   PASS=<PASS> scripts/android-adb.sh install    最新のリリース（aarch64 musl）を入れて起動する
#   scripts/android-adb.sh install                2回目以降は、入っている PASS のまま入れ直す
#   scripts/android-adb.sh install --binary <file> 手元でビルドしたものを入れる
#   scripts/android-adb.sh install --hostname <名前> PC Status に出す名前（既定は機種名。例: SH-M28）
#   scripts/android-adb.sh stop | status | log
#
# adb shell の権限で動くので、CPU（コアごと）・GPU（Adreno）・ロードアベレージも取れる。
# 端末から切り離して動かすので、ケーブルを抜いても・画面を消して Doze に入っても止まらない。
# 落ちたとき・自動更新で終わったときは 5 秒後に起動し直す。
# 端末を再起動すると止まるので、もう一度 install する（USB か、無線デバッグの adb connect で）。
# 端末が複数つながっているときは ANDROID_SERIAL で選ぶ。
set -eu

DIR=/data/local/tmp/pcsc-rs
TARGET=aarch64-unknown-linux-musl

stop_remote() {
  # ループ（PID はループ自身が loop.pid に書く）を先に止めてから、本体を止める
  adb shell "test -f $DIR/loop.pid && kill \$(cat $DIR/loop.pid) 2>/dev/null; rm -f $DIR/loop.pid; pkill -x pcsc-rs 2>/dev/null; true"
}

case "${1:-install}" in
  install)
    shift || true
    binary=
    hostname=
    while [ $# -gt 0 ]; do
      case "$1" in
        --binary) binary=${2:?--binary にファイルを指定して}; shift 2 ;;
        --hostname) hostname=${2:?--hostname に名前を指定して}; shift 2 ;;
        *) echo "知らないオプション: $1" >&2; exit 1 ;;
      esac
    done

    case "$(adb shell uname -m | tr -d '\r')" in
      aarch64) ;;
      *) echo "未対応のアーキテクチャ（aarch64 だけ）" >&2; exit 1 ;;
    esac

    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    if [ -z "$binary" ]; then
      tag=$(curl -fsSL https://api.github.com/repos/Zel9278/pcsc-rs/releases/latest | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p')
      [ -n "$tag" ] || { echo "最新リリースが取れなかった" >&2; exit 1; }
      if [ -t 2 ]; then progress=--progress-bar; else progress=-sS; fi
      curl -fL "$progress" -o "$tmp/pcsc-rs" "https://github.com/Zel9278/pcsc-rs/releases/download/$tag/pcsc-rs-$tag-$TARGET"
      binary=$tmp/pcsc-rs
    fi

    adb shell "mkdir -p $DIR"
    # 指定が無ければ、入っている .env の PASS と名前を引き継ぐ
    old_env=$(adb shell "cat $DIR/.env 2>/dev/null" | tr -d '\r')
    pass=${PASS:-$(printf '%s\n' "$old_env" | sed -n 's/^PASS=//p')}
    hostname=${hostname:-$(printf '%s\n' "$old_env" | sed -n 's/^HOSTNAME=//p')}
    [ -n "$pass" ] || { echo "PASS を指定して（PASS=<PASS> $0 install）" >&2; exit 1; }
    {
      printf 'PASS=%s\nPCSC_UPDATED=terminate\n' "$pass"
      [ -z "$hostname" ] || printf 'HOSTNAME=%s\n' "$hostname"
    } | adb shell "umask 077; cat > $DIR/.env"

    stop_remote
    adb push "$binary" "$DIR/pcsc-rs" >/dev/null
    adb shell "chmod 755 $DIR/pcsc-rs"
    # 本体が終わったら（落ちた・更新した）5 秒後に起動し直す。.env は作業フォルダから読まれる
    adb shell "cat > $DIR/loop.sh" <<'LOOP'
#!/system/bin/sh
cd "$(dirname "$0")" || exit 1
echo $$ > loop.pid
# adb shell は HOSTNAME に機種のコード名を入れてくる。.env の名前（無ければ機種名）を使わせる
unset HOSTNAME
while true; do
  ./pcsc-rs
  sleep 5
done
LOOP
    adb shell "chmod 755 $DIR/loop.sh"
    # サブシェルで起動して、adb shell の sh がすぐ終わるようにする（残ると adb が戻ってこない）
    adb shell "cd $DIR && (setsid $DIR/loop.sh > pcsc-rs.log 2>&1 < /dev/null &)"
    sleep 4
    adb shell "tail -n 6 $DIR/pcsc-rs.log"
    ;;
  stop)
    stop_remote
    echo "止めた"
    ;;
  status)
    adb shell "ps -A -o PID,USER,ETIME,ARGS | grep -E '^ *PID|pcsc-rs' | grep -v grep" || echo "動いていない"
    ;;
  log)
    adb shell "tail -n ${2:-30} $DIR/pcsc-rs.log"
    ;;
  *)
    echo "使い方: $0 [install [--binary <file>] | stop | status | log [行数]]" >&2
    exit 1
    ;;
esac
