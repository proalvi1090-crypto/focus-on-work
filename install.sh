#!/bin/sh
set -eu

PREFIX=${PREFIX:-/usr/local}
UNIT_DIR=${UNIT_DIR:-/etc/systemd/system}
echo "Building release binaries..."
cargo build --release --workspace
echo "Installing binaries to $PREFIX/bin..."
install -Dm755 target/release/focus-on-work-daemon "$PREFIX/bin/focus-on-work-daemon"
install -Dm755 target/release/focus-on-work "$PREFIX/bin/focus-on-work"
echo "Installing systemd unit..."
install -Dm644 packaging/focus-on-work.service "$UNIT_DIR/focus-on-work.service"
echo "Reloading systemd and enabling service..."
systemctl daemon-reload
systemctl enable --now focus-on-work.service
echo "focus-on-work installed and started."
