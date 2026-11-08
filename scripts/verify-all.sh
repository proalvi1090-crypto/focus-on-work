#!/bin/sh
set -eu

cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
echo
echo "Root/manual checks:"
echo "  scripts/verify-systemd.sh (systemd unit; root recommended)"
echo "  scripts/verify-socket.sh USER (root, installed service, unprivileged user)"
echo "  scripts/test-uninstall-hosts.sh (safe temporary-file regression)"
echo "  GUI runtime behavior needs a graphical Linux session."
