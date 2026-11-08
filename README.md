# focus-on-work

`focus-on-work` is a lightweight Linux app that blocks user-selected websites
and applications for a chosen duration, then unblocks them automatically.

The project is split into a core library, a root-owned daemon, a command line
client, and a Slint desktop GUI.

## Daemon

The daemon reads `FOW_HOSTS_PATH`, `FOW_STATE_PATH`, and `FOW_SOCKET_PATH`;
command-line flags `--hosts`, `--state`, and `--socket` override those
environment variables. Defaults target `/etc/hosts`, `/var/lib/focus-on-work`,
and `/run/focus-on-work/focus-on-work.sock`, respectively.

For safe local testing, use a temporary hosts file and socket:

```sh
mkdir -p /tmp/focus-on-work
printf '127.0.0.1 localhost\n' > /tmp/focus-on-work/hosts
FOW_HOSTS_PATH=/tmp/focus-on-work/hosts \
FOW_STATE_PATH=/tmp/focus-on-work/state.json \
FOW_SOCKET_PATH=/tmp/focus-on-work/daemon.sock \
cargo run -p focus-on-work-daemon
```

Editing `/etc/hosts`, creating the default state/socket locations, and
terminating processes owned by other users require root or equivalent
permissions. The daemon deliberately leaves an active hosts block in place
when it receives SIGINT or SIGTERM; it only removes it when the session
expires.

## Install

On a Linux system with Rust and systemd:

```sh
sudo ./install.sh
```

The installer builds release binaries, installs them under `/usr/local/bin`,
installs `packaging/focus-on-work.service`, and enables/starts the service.
Run `sudo ./uninstall.sh` to stop the service, remove the managed hosts block,
and remove installed files. Both scripts are safe to run more than once.

The unit runs as root because it must edit `/etc/hosts` and terminate processes
owned by other users. Verify these assumptions on a real system before relying
on the hardening configuration:

- `After=local-fs.target` starts after local filesystems are available.
- `User=root` permits hosts edits and process termination.
- `Group=focus-on-work` makes the runtime directory and socket available to
  members of the dedicated group; `RuntimeDirectoryMode=0770` lets them enter
  the directory.
- `Restart=always` and `RestartSec=2s` recover from daemon exits.
- `ProtectSystem=full` makes the rest of the filesystem read-only.
- `ReadWritePaths=/etc` allows hosts updates. `StateDirectory=focus-on-work`
  provides `/var/lib/focus-on-work`, and `RuntimeDirectory=focus-on-work`
  provides the socket directory.
- `ProtectHome=yes` hides users' home directories.
- `PrivateTmp=yes` gives the service a private temporary directory.
- `PrivateDevices=yes` hides unnecessary device nodes.
- Kernel, namespace, SUID, and personality restrictions reduce attack surface.
- `CapabilityBoundingSet` retains only capabilities needed for protected writes
  and process termination.

## Usage

```sh
focus-on-work start --duration 2h --sites youtube.com,facebook.com --apps firefox,steam
focus-on-work status
focus-on-work stop
```

`status` reports whether a session is active, its remaining time, and blocked
sites/apps. `stop` is intentionally refused while a session is active; the
daemon removes blocks when the timer expires. Set `FOW_SOCKET_PATH` or pass
`--socket` to use a non-default daemon socket. The CLI reports a friendly
error if the daemon is not running or the socket cannot be reached.

## Slint GUI

The GUI is the `focus-on-work-gui` workspace member. Build and run it with:

```sh
cargo run -p focus-on-work-gui
```

It connects to `/run/focus-on-work/focus-on-work.sock` by default. For local testing, point
both processes at the same temporary socket:

```sh
FOW_SOCKET_PATH=/tmp/focus-on-work/daemon.sock cargo run -p focus-on-work-gui
```

The GUI sends newline-delimited JSON `Start` and `Status` requests, polls
status once per second, and displays the daemon's authoritative session
expiry. `FOW_GUI_CONFIG` changes the JSON import/export path. Closing the
window hides it and sends a desktop notification; the daemon continues
enforcing an active session.

The GUI supports preset categories plus custom comma-separated website domains
and exact process names or executable paths. `Custom list only` disables the
presets for the next session and uses only the custom entries. Settings can be
locked before starting a session; this disables editing controls until
unlocked.

`deny.toml` permits the listed permissive licenses and explicitly allows
GPL-3.0-only; it does not claim that every copyleft license is denied. Run
`cargo deny check licenses` in CI after installing `cargo-deny`; Slint's own
licensing terms should also be reviewed against the project's Commons Clause
policy before distributing binaries.

## Limitations

- Hosts-file blocking cannot wildcard subdomains and can be bypassed by
  browsers using DNS-over-HTTPS.
- Snap/Flatpak process naming is untested.
- - The installer creates the `focus-on-work` group. Users must join it
  (`sudo usermod -aG focus-on-work USER`) and log in again before using the
  socket without sudo. Users outside the group are denied.
- A root user can always undo the hosts or process block.

## Safe manual test plan

Use a virtual machine or throwaway Linux installation. Back up `/etc/hosts`
first, then use temporary paths where possible:

```sh
cp /etc/hosts /tmp/hosts.backup
mkdir -p /tmp/focus-on-work
printf '127.0.0.1 localhost\n' > /tmp/focus-on-work/hosts
FOW_HOSTS_PATH=/tmp/focus-on-work/hosts \
FOW_STATE_PATH=/tmp/focus-on-work/state.json \
FOW_SOCKET_PATH=/tmp/focus-on-work/daemon.sock \
cargo run -p focus-on-work-daemon
```

In another terminal, run the three CLI commands with
`FOW_SOCKET_PATH=/tmp/focus-on-work/daemon.sock`, inspect the temporary hosts
file, send SIGTERM to the daemon, and confirm the active block remains. Test
expiry with a short duration such as `1m`. Do not test against a production
machine until the service unit's permissions and hardening have been reviewed.

## License

This project is source-available under the MIT License with an additional
Commons Clause condition. You may read, learn from, copy, modify, and use it
for personal, educational, research, and other non-commercial purposes.

You may not sell the software, include it in commercial software, offer paid
hosting or services based on it, or use it for a commercial business without
prior written permission from Kamran Ahmed Alvi (proalvi1090-crypto). The
copyright notice and complete license text must remain with copies or
substantial portions of the software.

Because commercial use is restricted, this is source-available and is not an
OSI-approved Open Source license. Consult a qualified lawyer before relying on
this custom license for commercial or redistribution decisions.
# focus-on-work
