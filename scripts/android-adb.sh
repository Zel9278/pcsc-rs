#!/bin/sh
# Android で pcsc-rs を動かし続ける（PC から adb で入れる）。root は要らない。
#
#   PASS=<PASS> scripts/android-adb.sh install    端末に合う最新リリースを入れて起動する
#   scripts/android-adb.sh install                2回目以降は、入っている PASS のまま入れ直す
#   scripts/android-adb.sh install --binary <file> 手元でビルドしたものを入れる
#   scripts/android-adb.sh install --hostname <名前> PC Status に出す名前（既定は機種名。例: SH-M28）
# PCSC_HIDE=<項目,…> scripts/android-adb.sh install PC Status に出さない項目（README の PCSC_HIDE）。
# 省くと前の設定のまま、PCSC_HIDE= で全部出す
#   scripts/android-adb.sh install --libc bionic  32bit ARM で Android 5.0 以降なら Android 標準の DNS を使う版を入れる
#   scripts/android-adb.sh stop | status | log [行数]
#   scripts/android-adb.sh devices                つながっている端末とシリアルの一覧
#   … --adb <adb の場所>（または環境変数 ADB）で、PATH に無い adb を使う
#   … --serial <シリアル>（または ANDROID_SERIAL）で端末を選ぶ。USB でも、無線デバッグの IP:ポートでもよい
#
# --libc は auto（既定）、bionic、musl から選ぶ。
#   auto   : 64bit ARM は bionic 版、32bit ARM は musl 版（従来どおり）
#   bionic : Android 5.0（API 21）以降用。musl 版が /etc/resolv.conf を探して名前解決に失敗する端末向け
#   musl   : 静的ビルド。Android 2.3.3 以降の 32bit ARM で使える
#
# adb shell の権限で動くので、CPU（コアごと）・GPU（Adreno）・ロードアベレージも取れる。
# 端末から切り離して動かすので、ケーブルを抜いても・画面を消して Doze に入っても止まらない。
# 落ちたときは 5 秒後に起動し直す。
# 端末を再起動すると止まるので、もう一度 install する（USB か、無線デバッグの adb connect で）。
# 端末が複数つながっているときは --serial か ANDROID_SERIAL で選ぶ（devices で一覧が出る）。
set -eu

DIR=/data/local/tmp/pcsc-rs
command=install
binary=
hostname=
libc=auto
lines=30
adb_bin=${ADB:-adb}
serial=${ANDROID_SERIAL:-}
while [ $# -gt 0 ]; do
  case "$1" in
    install | stop | status | log | devices) command=$1; shift ;;
    --binary) binary=${2:?--binary にファイルを指定して}; shift 2 ;;
    --hostname) hostname=${2:?--hostname に名前を指定して}; shift 2 ;;
    --libc) libc=${2:?--libc に auto、bionic、musl のどれかを指定して}; shift 2 ;;
    --adb) adb_bin=${2:?--adb に adb の場所を指定して}; shift 2 ;;
    --serial | -s) serial=${2:?--serial にシリアルを指定して}; shift 2 ;;
    [0-9]*) lines=$1; shift ;;
    *)
      echo "使い方: $0 [install [--binary <file>] [--hostname <名前>] [--libc auto|bionic|musl] | stop | status | log [行数] | devices] [--serial <シリアル>] [--adb <adb の場所>]" >&2
      exit 1
      ;;
  esac
done

case "$libc" in
  auto | bionic | musl) ;;
  *) echo "--libc は auto、bionic、musl のどれかを指定して（指定された値: $libc）" >&2; exit 1 ;;
esac

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
  # 古い toolbox には pidof や ps のオプションがないため、PID ファイルと
  # 標準の ps 出力（USER PID ... NAME）だけを使う。
  adb shell "for pidfile in $DIR/loop.pid $DIR/client.pid; do if test -f \$pidfile; then pid=\$(cat \$pidfile 2>/dev/null); case \$pid in ''|*[!0-9]*) ;; *) kill -9 \$pid 2>/dev/null ;; esac; rm \$pidfile 2>/dev/null || true; fi; done; ps | while read user pid ppid vsize rss wchan pc state name; do case \$name in pcsc-rs|*/pcsc-rs) kill -9 \$pid 2>/dev/null ;; esac; done; true"
}

# ログの末尾。Android 2.3 の toolbox には tail が無いことがあるので、PC 側で切る
remote_log() {
  adb shell "cat $DIR/pcsc-rs.log 2>/dev/null" | tr -d '\r' | tail -n "$1"
}

case "$command" in
  install)

    abi=$(adb shell getprop ro.product.cpu.abi | tr -d '\r')
    [ -n "$abi" ] || abi=$(adb shell uname -m | tr -d '\r')
    case "$abi" in
      aarch64 | arm64-v8a)
        # Android 5.0+ の arm64 には Android 標準（bionic）の API 21 ビルドしか無い
        [ "$libc" != musl ] || { echo "arm64 に musl 版は無い。--libc bionic か auto を使って" >&2; exit 1; }
        targets="aarch64-linux-android"
        ;;
      armv7l | armeabi-v7a)
        if [ "$libc" = bionic ]; then
          # Android 5.0（API 21）以降が必要
          targets="armv7-linux-androideabi"
        else
          # Android 2.3.3+ の 32bit ARM は静的 musl ビルドを使う
          targets="armv7-unknown-linux-musleabi"
        fi
        ;;
      armv6l | armeabi | arm)
        [ "$libc" != bionic ] || { echo "armeabi に bionic 版は無い。--libc musl か auto を使って" >&2; exit 1; }
        targets="arm-unknown-linux-musleabi"
        ;;
      *)
        echo "未対応のアーキテクチャ: $abi（aarch64、armeabi-v7a、armeabi に対応）" >&2
        exit 1
        ;;
    esac

    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    if [ -z "$binary" ]; then
      tag=$(curl -fsSL https://api.github.com/repos/SHINTRANET-Lab/pcsc-rs/releases/latest | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p')
      [ -n "$tag" ] || { echo "最新リリースが取れなかった" >&2; exit 1; }
      if [ -t 2 ]; then progress=--progress-bar; else progress=-sS; fi
      for target in $targets; do
        # 次の候補に進むのはファイルが無いとき（404）だけ。通信の失敗で musl 版に切り替わらないように
        code=$(curl -L "$progress" -o "$tmp/pcsc-rs" -w '%{http_code}' "https://github.com/SHINTRANET-Lab/pcsc-rs/releases/download/$tag/mimizk-rs-$tag-$target") || {
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
      [ -f "$tmp/pcsc-rs" ] || { echo "$tag に $targets のファイルが無い（$abi 向け）" >&2; exit 1; }
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
    case "$pass$hostname${PCSC_HIDE:-}" in *\'*) echo "PASS と名前に ' は使えない" >&2; exit 1 ;; esac
    # PCSC_HIDE を渡されたら入れ直す（空なら消す）。渡されなければ前のまま
    replace='PASS|PCSC_UPDATED|HOSTNAME'
    [ -z "${PCSC_HIDE+set}" ] || replace="$replace|PCSC_HIDE"
    {
      printf "PASS='%s'\n" "$pass"
      printf 'PCSC_UPDATED=terminate\n'
      [ -z "$hostname" ] || printf "HOSTNAME='%s'\n" "$hostname"
      [ -z "${PCSC_HIDE:-}" ] || printf "PCSC_HIDE='%s'\n" "$PCSC_HIDE"
      # ほかの設定（PCSC_URI、DEV_MODE など）はそのまま残す
      printf '%s\n' "$old_env" | grep -vE "^($replace)=|^\$" || true
    } | adb shell "umask 077; cat > $DIR/.env"

    stop_remote
    adb push "$binary" "$DIR/pcsc-rs" >/dev/null
    adb shell "chmod 755 $DIR/pcsc-rs"
    # 古い起動の跡を消してから、起動用のスクリプトを書く
    adb shell "rm -f $DIR/loop.sh $DIR/loop.pid $DIR/client.pid"
    # 本体が終わったら（落ちた・更新した）5 秒後に起動し直す。.env は作業フォルダから読まれる
    adb shell "cat > $DIR/loop.sh" <<LOOP
#!/system/bin/sh
trap '' HUP
export TMPDIR=$DIR
cd $DIR || exit 1
echo \$\$ > loop.pid
# adb shell は HOSTNAME に機種のコード名を入れてくる。.env の名前（無ければ機種名）を使わせる
unset HOSTNAME
export TMPDIR=/data/local/tmp/pcsc-rs
while true; do
  ./pcsc-rs &
  client_pid=\$!
  echo \$client_pid > client.pid
  wait \$client_pid
  rm -f client.pid
  sleep 5
done
LOOP
    adb shell "chmod 755 $DIR/loop.sh"
    # 今回の出力だけで接続を確かめられるよう、古いログを空にする
    adb shell "cd $DIR && : > pcsc-rs.log"

   sdk=$(adb shell getprop ro.build.version.sdk | tr -d '\r')
    redirect="> $DIR/pcsc-rs.log 2>&1 < /dev/null"
    if [ "${sdk:-0}" -ge 24 ] 2>/dev/null; then
      modern=1
      start_cmd="cd $DIR || exit 1; if command -v setsid >/dev/null 2>&1; then setsid nohup ./loop.sh $redirect & else nohup ./loop.sh $redirect & fi"
    else
      modern=0
      start_cmd="cd $DIR && ./loop.sh $redirect"
    fi
    if [ -n "$serial" ]; then
      command "$adb_bin" -s "$serial" shell "$start_cmd" > "$tmp/start.out" 2>&1 < /dev/null &
    else
      command "$adb_bin" shell "$start_cmd" > "$tmp/start.out" 2>&1 < /dev/null &
    fi
    start_pid=$!
    # loop.pid ができたのを確かめる（遅い端末もあるので最大 30 秒）
    started=0
    i=0
    while [ $i -lt 60 ]; do
      sleep 0.5 2>/dev/null || sleep 1
      i=$((i + 1))
      loop_pid=$(adb shell "cat $DIR/loop.pid 2>/dev/null; true" | tr -d '\r') || loop_pid=
      case "$loop_pid" in
        '' | *[!0-9]*) ;;
        *) started=1; break ;;
      esac
      # 古い端末の loop.sh は終わらないので、adb が終わっていたら起動に失敗している。
      # Android 7 以降は、起動し終えたシェルが終わるのが普通
      [ "$modern" -eq 1 ] || kill -0 "$start_pid" 2>/dev/null || break
    done
    # PC 側の adb だけを閉じる（端末の loop.sh は動き続ける）
    kill "$start_pid" 2>/dev/null || true
    wait "$start_pid" 2>/dev/null || true
    if [ "$started" -ne 1 ]; then
      echo "起動できなかった（loop.pid ができなかった）" >&2
      echo "[adb]" >&2
      cat "$tmp/start.out" >&2 2>/dev/null || true
      echo "[device]" >&2
      adb shell "ls -l $DIR" 2>&1 | tr -d '\r' >&2 || true
      remote_log 20 >&2 || true
      exit 1
    fi

    # つながるか、失敗か、断られるまで待つ（更新の確認に時間がかかる端末もあるので最大 20 秒）
    i=0
    while [ $i -lt 20 ]; do
      sleep 1
      i=$((i + 1))
      adb shell "cat $DIR/pcsc-rs.log 2>/dev/null" | grep -qE 'Received hi|Connection failed|refused' && break
    done
    remote_log 8
    ;;
  stop)
    stop_remote
    echo "止めた"
    ;;
  status)
    ps_out=$(adb shell "ps | while read user pid ppid vsize rss wchan pc state name; do case \$name in pcsc-rs|*/pcsc-rs) echo \$user \$pid \$ppid \$vsize \$rss \$wchan \$pc \$state \$name ;; esac; done" | tr -d '\r') || true
    if [ -n "$ps_out" ]; then
      printf '%s\n' "$ps_out"
    else
      echo "動いていない"
    fi
    ;;
  log)
    remote_log "$lines"
    ;;
esac
