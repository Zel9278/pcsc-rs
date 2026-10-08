#!/bin/sh
# pcsc-rs の最新リリースを Linux に入れて systemd に登録する
#   curl -fsSL https://raw.githubusercontent.com/Zel9278/pcsc-rs/main/install.sh | sudo PASS=<PASS> sh
#       … システム全体（/usr/local/bin、system のユニット）
#   curl -fsSL https://raw.githubusercontent.com/Zel9278/pcsc-rs/main/install.sh | PASS=<PASS> sh
#       … このユーザーだけ（~/.local/bin、systemd --user のユニット）
# 2回目以降は PASS を省くと、既に入っているユニットの PASS をそのまま使う。HOSTNAME などほかの Environment= も引き継ぐ
set -eu

if [ "$(id -u)" = 0 ]; then
  scope=system
  bin=/usr/local/bin/pcsc-rs
  unit=/etc/systemd/system/pcsc-rs.service
  other=${SUDO_USER:+$(getent passwd "$SUDO_USER" | cut -d: -f6)/.config/systemd/user/pcsc-rs.service}
  wanted=network-online.target
  systemctl() { command systemctl "$@"; }
  journal() { journalctl -u pcsc-rs "$@"; }
else
  scope=user
  bin=$HOME/.local/bin/pcsc-rs
  unit=${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/pcsc-rs.service
  other=/etc/systemd/system/pcsc-rs.service
  wanted=default.target
  systemctl() { command systemctl --user "$@"; }
  journal() { journalctl --user -u pcsc-rs "$@"; }
fi

# 同じホスト名のクライアントは1台しか繋がらない
if [ -n "$other" ] && [ -f "$other" ]; then
  echo "もう片方（$other）も入ってる。同じホスト名だと後から繋いだ方がサーバーに断られるので、どちらかを止めて" >&2
fi

case "$(uname -m)" in
  x86_64) target=x86_64-unknown-linux-musl ;;
  aarch64) target=aarch64-unknown-linux-musl ;;
  *) echo "未対応のアーキテクチャ: $(uname -m)" >&2; exit 1 ;;
esac

# 既に入っているユニットから PASS と、それ以外の設定（HOSTNAME など）を引き継ぐ。
# Environment="PASS=…" と Environment=PASS=… のどちらの書き方も読む
keep=
if [ -f "$unit" ]; then
  if [ -z "${PASS:-}" ]; then
    PASS=$(sed -n -e 's/^Environment="PASS=\(.*\)"$/\1/p' -e 's/^Environment=PASS=\([^"]*\)$/\1/p' "$unit" | head -n 1)
  fi
  keep=$(grep -E '^Environment=' "$unit" | grep -vE '^Environment="?(PASS|PCSC_UPDATED)=' || true)
fi
[ -n "${PASS:-}" ] || { echo "PASS を指定して（… | PASS=<PASS> sh）" >&2; exit 1; }

tag=$(curl -fsSL https://api.github.com/repos/Zel9278/pcsc-rs/releases/latest | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p')
[ -n "$tag" ] || { echo "最新リリースが取れなかった" >&2; exit 1; }

tmp=$(mktemp -d)
trap 'rm -f "$tmp/pcsc-rs"; rmdir "$tmp"' EXIT
# 進捗バーは端末で実行したときだけ
if [ -t 2 ]; then progress=--progress-bar; else progress=-sS; fi
curl -fL "$progress" -o "$tmp/pcsc-rs" "https://github.com/Zel9278/pcsc-rs/releases/download/$tag/pcsc-rs-$tag-$target"
install -D --no-target-directory -m 755 "$tmp/pcsc-rs" "$bin"

mkdir -p "$(dirname "$unit")"
umask 077
cat > "$unit" <<UNIT
[Unit]
Description=PCStatus Client
After=network-online.target

[Service]
Environment="PASS=$PASS"
Environment="PCSC_UPDATED=terminate"
${keep:+$keep
}ExecStart=$bin
Restart=always

[Install]
WantedBy=$wanted
UNIT

systemctl daemon-reload
systemctl enable pcsc-rs >/dev/null 2>&1
systemctl restart pcsc-rs
sleep 3
echo "入れた: $tag ($target, $scope)"
if [ "$scope" = user ] && [ "$(loginctl show-user "$(id -un)" -p Linger --value 2>/dev/null)" != yes ]; then
  echo "linger が無効なので、ログアウトすると止まる。常に動かすなら: loginctl enable-linger"
fi
journal -n 8 --no-pager -o cat
