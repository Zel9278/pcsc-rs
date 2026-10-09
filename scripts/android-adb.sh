#!/bin/sh
# Android で pcsc-rs を動かし続ける（PC から adb で入れる）。root は要らない。
#
#   PASS=<PASS> scripts/android-adb.sh install    最新のリリース（aarch64 musl）を入れて起動する
#   scripts/android-adb.sh install                2回目以降は、入っている PASS のまま入れ直す
#   scripts/android-adb.sh install --binary <file> 手元でビルドしたものを入れる
#   scripts/android-adb.sh install --hostname <名前> PC Status に出す名前（既定は機種名。例: SH-M28）
#   scripts/android-adb.sh stop | status | log [行数]
#   scripts/android-adb.sh devices                つながっている端末とシリアルの一覧
#   … --adb <adb の場所>（または環境変数 ADB）で、PATH に無い adb を使う
#   … --serial <シリアル>（または ANDROID_SERIAL）で端末を選ぶ。USB でも、無線デバッグの IP:ポートでもよい
#
# adb shell の権限で動くので、CPU（コアごと）・GPU（Adreno）・ロードアベレージも取れる。
# 端末から切り離して動かすので、ケーブルを抜いても・画面を消して Doze に入っても止まらない。
# 落ちたとき・自動更新で終わったときは 5 秒後に起動し直す。
# 端末を再起動すると止まるので、もう一度 install する（USB か、無線デバッグの adb connect で）。
# 端末が複数つながっているときは --serial か ANDROID_SERIAL で選ぶ（devices で一覧が出る）。
set -eu

DIR=/data/local/tmp/pcsc-rs
# Android 版（bionic）は Android の DNS を使える。それが無い古いリリースでは musl 版を使う
# （musl 版は /etc/resolv.conf を探すので、端末によっては DNS が引けない）
TARGETS="aarch64-linux-android aarch64-unknown-linux-musl"

command=install
binary=
hostname=
lines=30
adb_bin=${ADB:-adb}
serial=${ANDROID_SERIAL:-}
while [ $# -gt 0 ]; do
  case "$1" in
    install | stop | status | log | devices) command=$1; shift ;;
    --binary) binary=${2:?--binary にファイルを指定して}; shift 2 ;;
    --hostname) hostname=${2:?--hostname に名前を指定して}; shift 2 ;;
    --adb) adb_bin=${2:?--adb に adb の場所を指定して}; shift 2 ;;
    --serial | -s) serial=${2:?--serial にシリアルを指定して}; shift 2 ;;
    [0-9]*) lines=$1; shift ;;
    *)
      echo "使い方: $0 [install [--binary <file>] [--hostname <名前>] | stop | status | log [行数] | devices] [--serial <シリアル>] [--adb <adb の場所>]" >&2
      exit 1
      ;;
  esac
done

if ! command -v "$adb_bin" >/dev/null 2>&1; then
  echo "adb が見つからない: $adb_bin（--adb か環境変数 ADB で場所を指定して）" >&2
  exit 1
fi
if [ "$command" = devices ]; then
  command "$adb_bin" devices -l
  exit
fi

# 端末を選ぶ。指定が無ければ、つながっているのが1台だけのときにそれを使う
if [ -n "$serial" ]; then
  if [ "$(command "$adb_bin" -s "$serial" get-state 2>/dev/null)" != device ]; then
    echo "端末 $serial が使えない（つながっていないか、許可されていない）。つながっている端末:" >&2
    command "$adb_bin" devices | awk 'NR > 1 && NF >= 2' | sed 's/^/  /' >&2
    exit 1
  fi
else
  list=$(command "$adb_bin" devices | awk 'NR > 1 && NF >= 2')
  ready=$(printf '%s\n' "$list" | awk '$2 == "device"' | wc -l)
  if [ "$ready" -ne 1 ]; then
    if [ "$ready" -eq 0 ]; then
      echo "使える端末がつながっていない（USB デバッグか無線デバッグを有効にして、端末で許可して）" >&2
    else
      echo "端末が $ready 台つながっている。--serial で選んで:" >&2
    fi
    [ -z "$list" ] || printf '%s\n' "$list" | sed 's/^/  /' >&2
    printf '%s\n' "$list" | grep -q unauthorized && echo "（unauthorized の端末は、端末の画面でこの PC を許可して）" >&2
    exit 1
  fi
fi

# 以下の adb はすべて、指定された adb と端末を使う（command で関数自身を呼ばないようにする）
adb() {
  if [ -n "$serial" ]; then
    command "$adb_bin" -s "$serial" "$@"
  else
    command "$adb_bin" "$@"
  fi
}

stop_remote() {
  # ループ（PID はループ自身が loop.pid に書く）を先に止めてから、本体を止める
  adb shell "test -f $DIR/loop.pid && kill \$(cat $DIR/loop.pid) 2>/dev/null; rm -f $DIR/loop.pid; pkill -x pcsc-rs 2>/dev/null; true"
}

case "$command" in
  install)

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
      for target in $TARGETS; do
        # 次の候補に進むのはファイルが無いとき（404）だけ。通信の失敗で musl 版に切り替わらないように
        code=$(curl -L "$progress" -o "$tmp/pcsc-rs" -w '%{http_code}' "https://github.com/Zel9278/pcsc-rs/releases/download/$tag/pcsc-rs-$tag-$target") || {
          echo "ダウンロードに失敗した（$target）" >&2
          exit 1
        }
        if [ "$code" = 200 ]; then
          echo "$tag の $target を入れる"
          break
        fi
        rm -f "$tmp/pcsc-rs"
        [ "$code" = 404 ] || { echo "ダウンロードに失敗した（$target: HTTP $code）" >&2; exit 1; }
      done
      [ -f "$tmp/pcsc-rs" ] || { echo "$tag に aarch64 の Android 向けのファイルが無い" >&2; exit 1; }
      binary=$tmp/pcsc-rs
    fi

    adb shell "mkdir -p $DIR"
    # 指定が無ければ、入っている .env の PASS と名前を引き継ぐ
    old_env=$(adb shell "cat $DIR/.env 2>/dev/null" | tr -d '\r')
    # .env の値。'…' や "…" で囲んであれば外す
    env_value() {
      printf '%s\n' "$old_env" | sed -n "s/^$1=//p" | head -n 1 | sed -e "s/^'\\(.*\\)'\$/\\1/" -e 's/^"\(.*\)"$/\1/'
    }
    pass=${PASS:-$(env_value PASS)}
    hostname=${hostname:-$(env_value HOSTNAME)}
    [ -n "$pass" ] || { echo "PASS を指定して（PASS=<PASS> $0 install）" >&2; exit 1; }
    # 値は '…' で囲んで書く（空白や記号があってもそのまま読まれる）。' だけは使えない
    case "$pass$hostname" in *\'*) echo "PASS と名前に ' は使えない" >&2; exit 1 ;; esac
    {
      printf "PASS='%s'\nPCSC_UPDATED=terminate\n" "$pass"
      [ -z "$hostname" ] || printf "HOSTNAME='%s'\n" "$hostname"
      # ほかの設定（PCSC_URI、DEV_MODE など）はそのまま残す
      printf '%s\n' "$old_env" | grep -vE '^(PASS|PCSC_UPDATED|HOSTNAME)=|^$' || true
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
    # つながるか、失敗か、断られるまで待つ（更新の確認に時間がかかる端末もあるので最大 20 秒）
    i=0
    while [ $i -lt 20 ]; do
      sleep 1
      i=$((i + 1))
      adb shell "grep -qE 'Received hi|Connection failed|refused' $DIR/pcsc-rs.log" && break
    done
    adb shell "tail -n 8 $DIR/pcsc-rs.log"
    ;;
  stop)
    stop_remote
    echo "止めた"
    ;;
  status)
    adb shell "ps -A -o PID,USER,ETIME,ARGS | grep -E '^ *PID|pcsc-rs' | grep -v grep" || echo "動いていない"
    ;;
  log)
    adb shell "tail -n $lines $DIR/pcsc-rs.log"
    ;;
esac
