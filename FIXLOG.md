# Fix log

## 1. systemd unit

- Status: DONE
- Files changed: `packaging/focus-on-work.service`, `scripts/verify-systemd.sh`
- Tests added: none
- How verified: `systemd-analyze verify` is run by the verification script; repository checks run after this item.
- Unverified: a real root/systemd boot with `ProtectSystem=full` and `/etc/hosts` edits needs a VM or host manual run.
