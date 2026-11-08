#!/bin/sh
set -eu

# Run `cargo build -p focus-on-work-daemon` first. This test uses only /tmp.
dir=$(mktemp -d)
trap 'kill "$daemon_pid" 2>/dev/null || true; rm -rf "$dir"' EXIT
printf '127.0.0.1 localhost\n' > "$dir/hosts"
FOW_HOSTS_PATH="$dir/hosts" FOW_STATE_PATH="$dir/state.json" \
    FOW_SOCKET_PATH="$dir/daemon.sock" target/debug/focus-on-work-daemon >"$dir/log" 2>&1 &
daemon_pid=$!
sleep 0.2
python3 - "$dir/daemon.sock" <<'PY'
import socket
import sys
import time

path = sys.argv[1]
silent = socket.socket(socket.AF_UNIX)
silent.settimeout(3)
silent.connect(path)
silent.close()

abrupt = socket.socket(socket.AF_UNIX)
abrupt.connect(path)
abrupt.sendall(b'{"type":')
abrupt.close()

started = time.monotonic()
status = socket.socket(socket.AF_UNIX)
status.settimeout(1)
status.connect(path)
status.sendall(b'{"type":"Status"}\n')
response = status.recv(4096)
elapsed = time.monotonic() - started
assert b'"ok":true' in response, response
assert elapsed < 1.0, elapsed
print(f"status succeeded after {elapsed:.3f}s")
PY
