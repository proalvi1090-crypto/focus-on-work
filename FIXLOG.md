# Fix log

## 1. systemd unit

- Status: DONE
- Files changed: `packaging/focus-on-work.service`, `scripts/verify-systemd.sh`
- Tests added: none
- How verified: `systemd-analyze verify` is run by the verification script; repository checks run after this item.
- Unverified: a real root/systemd boot with `ProtectSystem=full` and `/etc/hosts` edits needs a VM or host manual run.

## 3. Socket permissions

- Status: DONE
- Files changed: daemon socket setup, daemon manifest, systemd unit, `scripts/verify-socket.sh`, `README.md`
- Tests added: commented VM acceptance script
- How verified: the daemon now relies on systemd `Group=focus-on-work` and
  `RuntimeDirectoryMode=0770`, sets only the socket mode, and falls back to
  `0600` with a warning when the group is absent. The unsafe non-NUL-terminated
  `libc::chown` call and unused libc dependency were removed; repository checks
  passed after this item.
- Unverified: real systemd ownership and an actual in-group/out-of-group
  client test require the manual script in a VM.

## 4. Process matching

- Status: DONE
- Files changed: core process matching, daemon/CLI/GUI validation
- Tests added: exact-name/path and decoy-process unit tests
- How verified: fmt, clippy with `-D warnings`, and all workspace tests passed; sysinfo 0.37.2 source confirms `refresh_processes(ProcessesToUpdate::All, true)`.
- Unverified: CPU impact for a real desktop workload was not measured here.

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

## 8. Process scanner reuse

- Status: DONE
- Files changed: `focus-on-work-core/src/process.rs`,
  `focus-on-work-daemon/src/main.rs`, `FIXLOG.md`
- Tests added: none
- How verified: the daemon now owns one `ProcessTerminator`, which reuses one
  `sysinfo::System` and refreshes process data on each scan. Repository checks
  passed. With an active temporary-path session, a 30-second `/proc` CPU sample
  measured `0.000%` (below the container's 10 ms jiffy resolution, so this is
  reported as less than roughly `0.033%` of one core).
- Unverified: this low-process sandbox measurement does not represent a busy
  desktop.

## 9. Cleanup: nested lockfile

- Status: DONE
- Files changed: `focus-on-work-core/Cargo.lock`, `FIXLOG.md`
- Tests added: none
- How verified: the workspace has only the root `Cargo.lock`; repository checks passed after removal.
- Unverified: none.

## 10. Minimum Rust version

- Status: DONE
- Files changed: all workspace `Cargo.toml` manifests and `README.md`
- Tests added: none
- How verified: the locked dependency graph reports Rust 1.92 as its highest
  minimum requirement, from Slint 1.18.1. Every package declares
  `rust-version = 1.92`; repository checks passed after this change.
- Unverified: building with an actual Rust 1.92 toolchain was not available in
  this environment.

## 11. Hosts backup refresh

- Status: DONE
- Files changed: `focus-on-work-core/src/hosts.rs`, `FIXLOG.md`
- Tests added: backup refresh after a user edit
- How verified: when no managed block is present, `.bak` is now refreshed only
  when its bytes differ from the current hosts file; existing-block updates
  still preserve the backup. Focused and workspace Rust checks passed.
- Unverified: none for temporary-file behavior.
