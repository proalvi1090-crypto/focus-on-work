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
    begin_count=$(grep -c '^# BEGIN focus-on-work$' "$HOSTS_PATH" || true)
    end_count=$(grep -c '^# END focus-on-work$' "$HOSTS_PATH" || true)
    if [ "$begin_count" -eq 1 ] && [ "$end_count" -eq 1 ]; then
        echo "CLI is unavailable; removing the complete marker block safely."
        tmp=$(mktemp)
        awk '
            $0 == "# BEGIN focus-on-work" { skip = 1; next }
            $0 == "# END focus-on-work" { skip = 0; next }
            !skip { print }
        ' "$HOSTS_PATH" > "$tmp"
        chmod --reference="$HOSTS_PATH" "$tmp" 2>/dev/null || true
        mv "$tmp" "$HOSTS_PATH"
    else
        echo "Warning: incomplete or ambiguous markers; leaving $HOSTS_PATH untouched."
    fi
fi
echo "Removing systemd unit and binaries..."
rm -f "$UNIT_DIR/focus-on-work.service" "$PREFIX/bin/focus-on-work" "$PREFIX/bin/focus-on-work-daemon"
systemctl daemon-reload
echo "focus-on-work uninstalled."
