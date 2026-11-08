use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use sysinfo::ProcessesToUpdate;
use sysinfo::{Signal, System};

const PROTECTED_NAMES: &[&str] = &[
    "systemd",
    "init",
    "sshd",
    "Xorg",
    "Xwayland",
    "gnome-shell",
    "plasmashell",
    "dbus-daemon",
    "gdm",
    "sddm",
    "lightdm",
    "focus-on-work",
    "focus-on-work-cli",
    "focus-on-work-gui",
    "focus-on-work-daemon",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessDescription {
    pub pid: u32,
    pub name: String,
    pub executable: Option<PathBuf>,
}

pub struct ProcessTerminator {
    system: System,
}

impl ProcessTerminator {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }

    pub fn terminate_matching(&mut self, entries: &[String], daemon_pid: Option<u32>) {
        let own_pid = std::process::id();
        if validate_app_entries(entries).is_err() {
            return;
        }
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        let initial_descriptions = process_descriptions(&self.system);
        for pid in matching_pids(entries, &initial_descriptions, own_pid, daemon_pid) {
            if let Some(process) = self.system.process(sysinfo::Pid::from_u32(pid)) {
                let _ = process.kill_with(Signal::Term);
            }
        }
        thread::sleep(Duration::from_millis(150));
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        for pid in matching_pids(
            entries,
            &process_descriptions(&self.system),
            own_pid,
            daemon_pid,
        ) {
            if let Some(process) = self.system.process(sysinfo::Pid::from_u32(pid)) {
                let _ = process.kill_with(Signal::Kill);
            }
        }
    }
}

impl Default for ProcessTerminator {
    fn default() -> Self {
        Self::new()
    }
}

pub fn validate_app_entry(entry: &str) -> Result<(), String> {
    if entry.len() < 3
        || entry.len() > 255
        || entry.chars().any(|character| character.is_control())
        || (!entry.contains('/') && PROTECTED_NAMES.contains(&entry))
    {
        return Err(format!("invalid or protected app entry: {entry}"));
    }
    if entry.contains('/') && entry.chars().any(|character| character == '\0') {
        return Err(format!("invalid app entry: {entry}"));
    }
    Ok(())
}

pub fn validate_app_entries(entries: &[String]) -> Result<(), String> {
    entries
        .iter()
        .try_for_each(|entry| validate_app_entry(entry))
}

pub fn matching_pids(
    entries: &[String],
    processes: &[ProcessDescription],
    own_pid: u32,
    daemon_pid: Option<u32>,
) -> Vec<u32> {
    processes
        .iter()
        .filter(|process| {
            process.pid != 1 && process.pid != own_pid && Some(process.pid) != daemon_pid
        })
        .filter(|process| {
            entries.iter().any(|entry| {
                if validate_app_entry(entry).is_err() {
                    return false;
                }
                if entry.contains('/') {
                    return process
                        .executable
                        .as_ref()
                        .is_some_and(|path| path.to_string_lossy() == entry.as_str());
                }
                process.name == *entry
                    || process
                        .executable
                        .as_ref()
                        .and_then(|path| path.file_name())
                        .is_some_and(|name| name.to_string_lossy() == entry.as_str())
            })
        })
        .map(|process| process.pid)
        .collect()
}

fn process_descriptions(system: &System) -> Vec<ProcessDescription> {
    system
        .processes()
        .values()
        .map(|process| ProcessDescription {
            pid: process.pid().as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            executable: process.exe().map(PathBuf::from),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: u32, name: &str, executable: Option<&str>) -> ProcessDescription {
        ProcessDescription {
            pid,
            name: name.into(),
            executable: executable.map(PathBuf::from),
        }
    }

    #[test]
    fn matches_exact_names_without_command_line_decoys() {
        let processes = vec![
            process(2, "fow-dummy", Some("/tmp/fow-dummy")),
            process(3, "fow-dummy-editor", Some("/tmp/fow-dummy-editor")),
            process(4, "sh", Some("/usr/bin/sh")),
            process(5, "shell", Some("/bin/sh")),
        ];
        assert_eq!(
            matching_pids(&["fow-dummy".into()], &processes, 99, None),
            vec![2]
        );
    }

    #[test]
    fn rejects_short_and_protected_entries() {
        assert!(validate_app_entry("e").is_err());
        assert!(validate_app_entry("systemd").is_err());
        assert!(validate_app_entry("focus-on-work-gui").is_err());
    }

    #[test]
    fn matches_full_executable_path() {
        let processes = vec![process(2, "dummy", Some("/opt/fow/dummy"))];
        assert_eq!(
            matching_pids(&["/opt/fow/dummy".into()], &processes, 99, None),
            vec![2]
        );
        assert!(matching_pids(&["dummy".into()], &processes, 99, None).contains(&2));
    }
}
