#!/bin/sh
# pcsc-rs の最新リリースを入れて、常に動くように登録する（Linux は systemd、macOS は launchd）。
# Nix（NixOS・nix-darwin・home-manager）なら flake を使う: https://github.com/Zel9278/pcsc-rs#nix
#   curl -fsSL https://raw.githubusercontent.com/Zel9278/pcsc-rs/main/install.sh | sudo PASS=<PASS> sh
#       … システム全体（/usr/local/bin。Linux は system のユニット、macOS は LaunchDaemon）
#   curl -fsSL https://raw.githubusercontent.com/Zel9278/pcsc-rs/main/install.sh | PASS=<PASS> sh
#       … このユーザーだけ（~/.local/bin。Linux は systemd --user、macOS は LaunchAgent）
# 2回目以降は PASS を省くと、既に入っている設定の PASS をそのまま使う。HOSTNAME などほかの設定も引き継ぐ
set -eu

os=$(uname -s)
arch=$(uname -m)
case "$os/$arch" in
  Linux/x86_64) target=x86_64-unknown-linux-musl ;;
  Linux/aarch64) target=aarch64-unknown-linux-musl ;;
  Darwin/arm64) target=aarch64-apple-darwin ;;
  Darwin/x86_64) target=x86_64-apple-darwin ;;
  *) echo "未対応の環境: $os $arch" >&2; exit 1 ;;
esac

if [ "$(id -u)" = 0 ]; then scope=system; else scope=user; fi

# NixOS の /etc/systemd は設定から作られるので書き換えられない。モジュールを使ってもらう
if [ -e /etc/NIXOS ]; then
  if [ "$scope" = system ]; then
    echo "NixOS では flake の NixOS モジュール（services.pcsc-rs）で入れて: https://github.com/Zel9278/pcsc-rs#nix" >&2
    exit 1
  fi
  echo "NixOS なので、systemd --user に入れる。home-manager を使っているならモジュールのほうが楽: https://github.com/Zel9278/pcsc-rs#nix" >&2
fi
if [ "$scope" = system ]; then bin=/usr/local/bin/pcsc-rs; else bin=$HOME/.local/bin/pcsc-rs; fi

# ---- 既にある設定の場所と、そこから引き継ぐ値 ----

if [ "$os" = Darwin ]; then
  label=io.github.zel9278.pcsc-rs
  if [ "$scope" = system ]; then
    conf=/Library/LaunchDaemons/$label.plist
    other=${SUDO_USER:+$(eval echo "~$SUDO_USER")/Library/LaunchAgents/$label.plist}
    log=/var/log/pcsc-rs.log
  else
    conf=$HOME/Library/LaunchAgents/$label.plist
    other=/Library/LaunchDaemons/$label.plist
    log=$HOME/Library/Logs/pcsc-rs.log
  fi
  plist_env() { /usr/libexec/PlistBuddy -c "Print :EnvironmentVariables:$1" "$conf" 2>/dev/null || true; }
  keep_env=
  if [ -f "$conf" ]; then
    [ -n "${PASS:-}" ] || PASS=$(plist_env PASS)
    for key in HOSTNAME DEV_MODE PCSC_URI; do
      value=$(plist_env "$key")
      [ -n "$value" ] && keep_env="$keep_env$key=$value
"
    done
  fi
else
  if [ "$scope" = system ]; then
    conf=/etc/systemd/system/pcsc-rs.service
    other=${SUDO_USER:+$(getent passwd "$SUDO_USER" | cut -d: -f6)/.config/systemd/user/pcsc-rs.service}
  else
    conf=${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user/pcsc-rs.service
    other=/etc/systemd/system/pcsc-rs.service
  fi
  # Environment="PASS=…" と Environment=PASS=… のどちらの書き方も読む
  keep=
  if [ -f "$conf" ]; then
    if [ -z "${PASS:-}" ]; then
      PASS=$(sed -n -e 's/^Environment="PASS=\(.*\)"$/\1/p' -e 's/^Environment=PASS=\([^"]*\)$/\1/p' "$conf" | head -n 1)
    fi
    keep=$(grep -E '^Environment=' "$conf" | grep -vE '^Environment="?(PASS|PCSC_UPDATED)=' || true)
  fi
fi

# 同じホスト名のクライアントは1台しか繋がらない
if [ -n "$other" ] && [ -f "$other" ]; then
  echo "もう片方（$other）も入ってる。同じホスト名だと後から繋いだ方がサーバーに断られるので、どちらかを止めて" >&2
fi
[ -n "${PASS:-}" ] || { echo "PASS を指定して（… | PASS=<PASS> sh）" >&2; exit 1; }

# ---- ダウンロード ----

tag=$(curl -fsSL https://api.github.com/repos/Zel9278/pcsc-rs/releases/latest | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p')
[ -n "$tag" ] || { echo "最新リリースが取れなかった" >&2; exit 1; }

tmp=$(mktemp -d)
trap 'rm -f "$tmp/pcsc-rs"; rmdir "$tmp"' EXIT
# 進捗バーは端末で実行したときだけ
if [ -t 2 ]; then progress=--progress-bar; else progress=-sS; fi
curl -fL "$progress" -o "$tmp/pcsc-rs" "https://github.com/Zel9278/pcsc-rs/releases/download/$tag/pcsc-rs-$tag-$target"
# 動いている古いファイルは書き換えず、置き換える（macOS では上書きするとコード署名の確認で落ちる）。
# 置き先で作り直すので、SELinux のラベルも置き先のものになる
mkdir -p "$(dirname "$bin")"
cp "$tmp/pcsc-rs" "$bin.new"
chmod 755 "$bin.new"
mv -f "$bin.new" "$bin"

# ---- 登録して起動 ----

if [ "$os" = Darwin ]; then
  xml() { printf %s "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g'; }
  envs=
  while IFS= read -r line; do
    [ -n "$line" ] || continue
    envs="$envs    <key>$(xml "${line%%=*}")</key><string>$(xml "${line#*=}")</string>
"
  done <<KEEP
$keep_env
KEEP
  mkdir -p "$(dirname "$conf")" "$(dirname "$log")"
  umask 077
  cat > "$conf" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$label</string>
  <key>ProgramArguments</key>
  <array><string>$(xml "$bin")</string></array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PASS</key><string>$(xml "$PASS")</string>
    <key>PCSC_UPDATED</key><string>terminate</string>
$envs  </dict>
  <!-- 起動時に動かし、終了したら（更新のあとも）起動し直す -->
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ThrottleInterval</key><integer>10</integer>
  <key>StandardOutPath</key><string>$(xml "$log")</string>
  <key>StandardErrorPath</key><string>$(xml "$log")</string>
</dict>
</plist>
PLIST
  if [ "$scope" = system ]; then
    # LaunchDaemon は root:wheel・644 でないと読み込まれない
    chown root:wheel "$conf"
    chmod 644 "$conf"
    domain=system
  else
    # GUI でログインしていれば gui/、ssh だけなら user/
    domain=gui/$(id -u)
    launchctl print "$domain" >/dev/null 2>&1 || domain=user/$(id -u)
  fi
  launchctl bootout "$domain/$label" 2>/dev/null || true
  launchctl bootstrap "$domain" "$conf"
  launchctl enable "$domain/$label"
  launchctl kickstart -k "$domain/$label"
  sleep 3
  echo "入れた: $tag ($target, $scope, launchd: $domain/$label)"
  tail -n 8 "$log" 2>/dev/null || true
else
  if [ "$scope" = system ]; then
    wanted=network-online.target
    systemctl() { command systemctl "$@"; }
    journal() { journalctl -u pcsc-rs "$@"; }
  else
    wanted=default.target
    systemctl() { command systemctl --user "$@"; }
    journal() { journalctl --user -u pcsc-rs "$@"; }
  fi
  mkdir -p "$(dirname "$conf")"
  umask 077
  cat > "$conf" <<UNIT
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
fi
