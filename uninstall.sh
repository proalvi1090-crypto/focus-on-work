#!/bin/sh
set -eu

PREFIX=${PREFIX:-/usr/local}
UNIT_DIR=${UNIT_DIR:-/etc/systemd/system}
HOSTS_PATH=${FOW_HOSTS_PATH:-/etc/hosts}
echo "Stopping and disabling focus-on-work.service (if present)..."
systemctl disable --now focus-on-work.service 2>/dev/null || true
echo "Removing any managed hosts block from $HOSTS_PATH..."
if [ -x "$PREFIX/bin/focus-on-work" ] && [ -f "$HOSTS_PATH" ]; then
    FOW_HOSTS_PATH="$HOSTS_PATH" "$PREFIX/bin/focus-on-work" cleanup || true
elif [ -f "$HOSTS_PATH" ]; then
    echo "CLI is unavailable; removing a complete marker range directly."
    tmp=$(mktemp)
    sed '/^# BEGIN focus-on-work$/,/^# END focus-on-work$/d' "$HOSTS_PATH" > "$tmp"
    chmod --reference="$HOSTS_PATH" "$tmp" 2>/dev/null || true
    mv "$tmp" "$HOSTS_PATH"
fi
echo "Removing systemd unit and binaries..."
rm -f "$UNIT_DIR/focus-on-work.service" "$PREFIX/bin/focus-on-work" "$PREFIX/bin/focus-on-work-daemon"
systemctl daemon-reload
echo "focus-on-work uninstalled."
