use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use sysinfo::{Signal, System};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessDescription {
    pub pid: u32,
    pub name: String,
    pub executable: Option<PathBuf>,
    pub command_line: String,
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
                process.name.contains(entry)
                    || process
                        .executable
                        .as_ref()
                        .is_some_and(|path| path.to_string_lossy().contains(entry))
                    || process.command_line.contains(entry)
            })
        })
        .map(|process| process.pid)
        .collect()
}

pub fn terminate_matching(entries: &[String], daemon_pid: Option<u32>) {
    let own_pid = std::process::id();
    let mut system = System::new_all();
    system.refresh_all();
    let descriptions = system
        .processes()
        .values()
        .map(|process| ProcessDescription {
            pid: process.pid().as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            executable: process.exe().map(PathBuf::from),
            command_line: process
                .cmd()
                .iter()
                .map(|part| part.to_string_lossy())
                .collect::<Vec<_>>()
                .join(" "),
        })
        .collect::<Vec<_>>();
    for pid in matching_pids(entries, &descriptions, own_pid, daemon_pid) {
        if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
            let _ = process.kill_with(Signal::Term);
        }
    }
    thread::sleep(Duration::from_millis(150));
    system.refresh_all();
    for pid in matching_pids(
        entries,
        &system
            .processes()
            .values()
            .map(|process| ProcessDescription {
                pid: process.pid().as_u32(),
                name: process.name().to_string_lossy().into_owned(),
                executable: process.exe().map(PathBuf::from),
                command_line: process
                    .cmd()
                    .iter()
                    .map(|part| part.to_string_lossy())
                    .collect::<Vec<_>>()
                    .join(" "),
            })
            .collect::<Vec<_>>(),
        own_pid,
        daemon_pid,
    ) {
        if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
            let _ = process.kill_with(Signal::Kill);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn process(pid: u32, name: &str, command_line: &str) -> ProcessDescription {
        ProcessDescription {
            pid,
            name: name.into(),
            executable: None,
            command_line: command_line.into(),
        }
    }

    #[test]
    fn matches_name_and_command_without_reserved_pids() {
        let processes = vec![
            process(2, "browser", ""),
            process(3, "shell", "blocked-app --x"),
            process(1, "browser", ""),
            process(4, "daemon", ""),
        ];
        assert_eq!(
            matching_pids(
                &["browser".into(), "blocked-app".into()],
                &processes,
                4,
                Some(4)
            ),
            vec![2, 3]
        );
    }
}
