#!/bin/sh
set -eu

tmp_dir=$(mktemp -d)
trap 'rm -rf "$tmp_dir"' EXIT
hosts="$tmp_dir/hosts"
cat > "$hosts" <<'EOF'
127.0.0.1 localhost
# BEGIN focus-on-work
keep this later line
127.0.0.1 should-remain
EOF
before=$(cat "$hosts")
HOSTS_PATH="$hosts" PREFIX="$tmp_dir/missing" UNIT_DIR="$tmp_dir/unit" \
    ./uninstall.sh >/dev/null 2>&1 || true
after=$(cat "$hosts")
test "$before" = "$after"
echo "incomplete-marker hosts file was left untouched"
