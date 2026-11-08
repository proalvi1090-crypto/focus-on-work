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

## 4. Process matching

- Status: DONE
- Files changed: core process matching, daemon/CLI/GUI validation
- Tests added: exact-name/path and decoy-process unit tests
- How verified: fmt, clippy with `-D warnings`, and all workspace tests passed; sysinfo 0.37.2 source confirms `refresh_processes(ProcessesToUpdate::All, true)`.
- Unverified: CPU impact was not measured.

## 5. Install and uninstall scripts

- Status: DONE
- Files changed: `install.sh`, `uninstall.sh`, `.gitignore`, removed root JSON config, `scripts/test-uninstall-hosts.sh`
- Tests added: incomplete-marker uninstall regression script
- How verified: script is isolated to a temporary hosts file; repository checks run after this item.
- Unverified: real package installation and systemd activation need a root/manual VM run.
