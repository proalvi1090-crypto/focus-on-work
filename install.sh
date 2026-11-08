#!/bin/sh
set -eu

PREFIX=${PREFIX:-/usr/local}
UNIT_DIR=${UNIT_DIR:-/etc/systemd/system}
echo "Building release binaries..."
if command -v cargo >/dev/null 2>&1; then
    cargo build --release -p focus-on-work-daemon -p focus-on-work-cli
else
    echo "cargo was not found in root's PATH."
    echo "Build with cargo as your normal user first, or install Rust for this user."
    test -x target/release/focus-on-work-daemon
    test -x target/release/focus-on-work
fi
echo "Installing binaries to $PREFIX/bin..."
install -Dm755 target/release/focus-on-work-daemon "$PREFIX/bin/focus-on-work-daemon"
install -Dm755 target/release/focus-on-work "$PREFIX/bin/focus-on-work"
if ! getent group focus-on-work >/dev/null 2>&1; then
    groupadd --system focus-on-work
    echo "Created group focus-on-work. Add users with: sudo usermod -aG focus-on-work USER"
    echo "Users must log in again before the CLI/GUI can access the socket."
fi
echo "Installing systemd unit..."
install -Dm644 packaging/focus-on-work.service "$UNIT_DIR/focus-on-work.service"
echo "Reloading systemd and enabling service..."
systemctl daemon-reload
systemctl enable --now focus-on-work.service
echo "focus-on-work installed and started."
