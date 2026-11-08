# Fix log

## 1. systemd unit

- Status: DONE
- Files changed: `packaging/focus-on-work.service`, `scripts/verify-systemd.sh`
- Tests added: none
- How verified: `systemd-analyze verify` is run by the verification script; repository checks run after this item.
- Unverified: a real root/systemd boot with `ProtectSystem=full` and `/etc/hosts` edits needs a VM or host manual run.

## 3. Socket permissions

- Status: PARTIAL
- Files changed: daemon socket setup, CLI/GUI default socket path, systemd runtime directory, `install.sh`, `scripts/verify-socket.sh`
- Tests added: none
- How verified: code uses mode `0660`, resolves the fixed `focus-on-work` group, and falls back to `0600`; repository checks are run after this item.
- Unverified: root/systemd and an actual unprivileged user/group membership test require the manual script and a VM/host.
