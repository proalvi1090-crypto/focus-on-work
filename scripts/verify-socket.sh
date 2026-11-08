#!/bin/sh
set -eu

# Needs root and an existing unprivileged user. Use a disposable VM because
# this starts a root daemon and terminates matching processes.
user=${1:?usage: $0 UNPRIVILEGED_USER}
echo "Manual check: start the daemon with temporary paths, then run:"
echo "  sudo -u $user FOW_SOCKET_PATH=/run/focus-on-work/focus-on-work.sock focus-on-work status"
echo "Confirm the command succeeds only after $user is in group focus-on-work."
echo "The installed systemd service should be used for the real permission check."
