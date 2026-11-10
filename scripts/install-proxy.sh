#!/bin/bash
set -euo pipefail

SOURCE="$(cd "$(dirname "$0")/../service" && pwd)"
DEST="/Library/Application Support/PortMap/Proxy"
PLIST="/Library/LaunchDaemons/com.portmap.proxy.plist"
CADDY="${1:-$(command -v caddy || true)}"

if [[ -z "$CADDY" ]]; then
  echo 'Install Caddy first: brew install caddy' >&2
  exit 1
fi

# Finish config validation before requesting administrator authentication.
PORTMAP_BIND='localhost 127.0.0.1 [::1]' "$CADDY" validate --config "$SOURCE/Caddyfile" --adapter caddyfile

if [[ $EUID -ne 0 ]]; then
  exec sudo /bin/bash "$0" "$CADDY"
fi

# The daemon runs as root to bind port 80. Copy its executable and configuration
# into a root-owned directory rather than running user-writable project files.
/bin/mkdir -p "$DEST" /var/log/portmap
/usr/sbin/chown root:wheel "$DEST" /var/log/portmap
/bin/chmod 755 "$DEST" /var/log/portmap
/bin/launchctl bootout system/com.portmap.proxy 2>/dev/null || true
/usr/bin/install -o root -g wheel -m 755 "$CADDY" "$DEST/caddy"
/usr/bin/install -o root -g wheel -m 644 "$SOURCE/Caddyfile" "$DEST/Caddyfile"
/usr/bin/install -o root -g wheel -m 644 "$SOURCE/start.py" "$DEST/start.py"

/bin/cat > "$PLIST" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>com.portmap.proxy</string>
  <key>ProgramArguments</key><array>
    <string>/usr/bin/python3</string><string>/Library/Application Support/PortMap/Proxy/start.py</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>ThrottleInterval</key><integer>15</integer>
  <key>WorkingDirectory</key><string>/Library/Application Support/PortMap/Proxy</string>
  <key>StandardOutPath</key><string>/var/log/portmap/proxy.log</string>
  <key>StandardErrorPath</key><string>/var/log/portmap/proxy-error.log</string>
</dict></plist>
PLIST
/usr/sbin/chown root:wheel "$PLIST"
/bin/chmod 644 "$PLIST"
/bin/launchctl enable system/com.portmap.proxy
/bin/launchctl bootstrap system "$PLIST"
echo 'PortMap proxy installed. Open http://3000.localhost for localhost:3000.'
