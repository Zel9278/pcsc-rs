#!/bin/sh
# Synology NAS（DSM 7）で pcsc-rs を動かし続ける。SSH は要らない。
#
# DSM の「コントロールパネル」→「タスクスケジューラ」→「作成」→「トリガーされたタスク」→
# 「ユーザー指定のスクリプト」で、ユーザーを root、イベントを「ブートアップ」にして、
# 「タスク設定」のスクリプトに次の1行を書く（作ったら右クリックの「実行」ですぐ動く）。
#
#   curl -fsSL https://raw.githubusercontent.com/Zel9278/pcsc-rs/main/scripts/synology.sh | PASS=<PASS> sh
#
#   PCSC_HOSTNAME=<名前>  PC Status に出す名前（既定は NAS のホスト名。bash が HOSTNAME を勝手に
#                         入れるので、この名前で渡す）
#   PCSC_URI=<URL>        ほかの PC Status サーバーへ送るとき
#   PCSC_HIDE=<項目,…>     PC Status に出さない項目（README の PCSC_HIDE。例: gpu_usage,temperature）
#   PCSC_DIR=<場所>       置き場所（既定は /volume1/pcsc-rs。DSM の更新で消えないようデータボリュームに置く）
#
# 2回目からは PASS などを省いてもよい（置き場所の .env を使う）。落ちたとき・自動更新で終わったときは
# 5 秒後に起動し直す。何度実行しても、前に動かしたものを止めてから1つだけ動かす。
# 止めるときは、同じスクリプトを PCSC_STOP=1 を付けて実行する。
set -eu

DIR=${PCSC_DIR:-/volume1/pcsc-rs}
mkdir -p "$DIR"
cd "$DIR"

# 前にこのスクリプトで動かしたループとクライアントだけを止める（ほかの pcsc-rs には触らない）。
# loop.pid を消すと、kill できなくてもループは次の周で終わる
for pid_file in loop.pid pcsc-rs.pid; do
  if [ -f "$pid_file" ]; then
    kill "$(cat "$pid_file")" 2>/dev/null || true
    rm -f "$pid_file"
  fi
done
if [ "${PCSC_STOP:-}" = 1 ]; then
  echo "stopped"
  exit 0
fi

case "$(uname -m)" in
  x86_64) target=x86_64-unknown-linux-musl ;;
  aarch64) target=aarch64-unknown-linux-musl ;;
  *)
    echo "この CPU（$(uname -m)）向けのビルドはありません" >&2
    exit 1
    ;;
esac

# 入っていなければ最新のリリースを入れる。以後の更新は pcsc-rs が自分で行う
if [ ! -x pcsc-rs ]; then
  url=$(curl -fsSL https://api.github.com/repos/Zel9278/pcsc-rs/releases/latest |
    grep -o "https://[^\"]*/pcsc-rs-v[^\"]*-$target\"" | tr -d '"' | head -n 1)
  [ -n "$url" ] || { echo "リリースに $target のファイルが見つかりません" >&2; exit 1; }
  curl -fsSLo pcsc-rs.new "$url"
  chmod 755 pcsc-rs.new
  mv -f pcsc-rs.new pcsc-rs
fi

# 渡された設定を .env に残す（次からは省ける）。値は単一引用符で囲むので、展開されない
if [ -n "${PASS:-}" ]; then
  umask 077
  {
    printf "PASS='%s'\n" "$PASS"
    [ -z "${PCSC_HOSTNAME:-}" ] || printf "HOSTNAME='%s'\n" "$PCSC_HOSTNAME"
    [ -z "${PCSC_URI:-}" ] || printf "PCSC_URI='%s'\n" "$PCSC_URI"
    [ -z "${PCSC_HIDE:-}" ] || printf "PCSC_HIDE='%s'\n" "$PCSC_HIDE"
    # 自分で更新したら終わる。ループが新しい版を起動する
    printf "PCSC_UPDATED=terminate\n"
  } > .env
  umask 022
fi
[ -f .env ] || { echo "PASS を指定してください（PASS=<PASS> を付けて実行）" >&2; exit 1; }

cat > loop.sh <<'EOF'
#!/bin/sh
cd "$(dirname "$0")" || exit 1
echo $$ > loop.pid
# 名前は .env（無ければ NAS のホスト名）から。シェルの HOSTNAME は使わない
unset HOSTNAME
while [ "$(cat loop.pid 2>/dev/null)" = "$$" ]; do
  # ログが大きくなりすぎないよう、1 MB を超えたら1世代だけ残す
  if [ "$(wc -c < pcsc-rs.log 2>/dev/null || echo 0)" -gt 1048576 ]; then
    mv -f pcsc-rs.log pcsc-rs.log.1
  fi
  ./pcsc-rs >> pcsc-rs.log 2>&1 &
  echo $! > pcsc-rs.pid
  wait $!
  sleep 5
done
EOF
chmod 755 loop.sh

nohup ./loop.sh > /dev/null 2>&1 &
echo "started in $DIR (log: $DIR/pcsc-rs.log)"
