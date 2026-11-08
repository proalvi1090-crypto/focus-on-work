#!/bin/sh
set -eu

# Needs root for a complete installation check. systemd-analyze verify itself
# is safe to run without root and validates the unit syntax and dependencies.
unit=${1:-packaging/focus-on-work.service}
if [ -x /usr/local/bin/focus-on-work-daemon ]; then
    systemd-analyze verify "$unit"
else
    temp=$(mktemp --suffix=.service)
    trap 'rm -f "$temp"' EXIT
    sed 's#^ExecStart=.*#ExecStart=/bin/true#' "$unit" > "$temp"
    systemd-analyze verify "$temp"
fi
echo "Unit verification passed. A root/systemd VM run is still needed to verify"
echo "the protected /etc write and socket lifecycle."
