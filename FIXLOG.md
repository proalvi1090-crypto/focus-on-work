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

## 2. Daemon robustness follow-up

- Status: DONE
- Files changed: `scripts/test-daemon-clients.sh`
- Tests added: silent-client, abrupt-disconnect, and fast-status integration script using temporary paths
- How verified: `cargo build -p focus-on-work-daemon && scripts/test-daemon-clients.sh` passed with a 0.045s status response; Rust checks pass.
- Unverified: none for the temporary-path client scenario.

## 5. Install and uninstall scripts

- Status: DONE
- Files changed: `install.sh`, `uninstall.sh`, `.gitignore`, removed root JSON config, `scripts/test-uninstall-hosts.sh`
- Tests added: incomplete-marker uninstall regression script
- How verified: script is isolated to a temporary hosts file; repository checks run after this item.
- Unverified: real package installation and systemd activation need a root/manual VM run.

## 6. Repository hygiene

- Status: PARTIAL
- Files changed: `.gitignore`, tracked `Cargo.lock`, GUI config path, CI dependency step, README, `scripts/verify-all.sh`
- Tests added: none
- How verified: workspace checks pass; GUI builds with the installed Linux libraries.
- Unverified: exact package requirements for every tray backend/platform were not independently confirmed from upstream crate documentation.

## 7. GUI

- Status: PARTIAL
- Files changed: GUI request dispatch, timeouts, config path, lock wording, generated tray icon
- Tests added: existing pure helper tests continue to pass
- How verified: fmt, clippy, and workspace tests pass; socket calls now run on worker threads and UI updates use `invoke_from_event_loop`.
- Unverified: Unix socket connect itself has no stable std connect-timeout API in this toolchain, so only read/write timeouts are enforced; real GUI/tray behavior needs a graphical session.
