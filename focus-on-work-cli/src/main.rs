use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use focus_on_work_core::hosts;
use focus_on_work_core::session::Session;
use serde::{Deserialize, Serialize};

#[derive(Debug, Parser)]
#[command(name = "focus-on-work", about = "Control the focus-on-work daemon")]
struct Cli {
    #[arg(
        long,
        env = "FOW_SOCKET_PATH",
        default_value = "/run/focus-on-work.sock",
        global = true
    )]
    socket: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Start {
        #[arg(long)]
        duration: String,
        #[arg(long, value_delimiter = ',')]
        sites: Vec<String>,
        #[arg(long, value_delimiter = ',')]
        apps: Vec<String>,
    },
    Status,
    Stop,
    #[command(hide = true)]
    Cleanup {
        #[arg(long, env = "FOW_HOSTS_PATH", default_value = "/etc/hosts")]
        hosts_path: PathBuf,
    },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum Request {
    Start {
        duration: String,
        sites: Vec<String>,
        apps: Vec<String>,
    },
    Status,
    Stop,
}

#[derive(Debug, Deserialize)]
struct Response {
    ok: bool,
    error: Option<String>,
    session: Option<Session>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("focus-on-work: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    let command = cli.command;
    if let Command::Cleanup { hosts_path } = command {
        hosts::remove_block(&hosts_path)
            .map_err(|error| format!("could not remove hosts block: {error}"))?;
        println!("Removed the focus-on-work hosts block (if present).");
        return Ok(());
    }

    let is_status = matches!(command, Command::Status);
    let request = match command {
        Command::Start {
            duration,
            sites,
            apps,
        } => Request::Start {
            duration,
            sites: split_entries(sites, "site")?,
            apps: split_entries(apps, "app")?,
        },
        Command::Status => Request::Status,
        Command::Stop => Request::Stop,
        Command::Cleanup { .. } => unreachable!(),
    };
    let response = send_request(&cli.socket, request)?;
    if !response.ok {
        return Err(response
            .error
            .unwrap_or_else(|| "daemon rejected the request".into()));
    }
    match response.session {
        Some(session) => print_session(&session),
        None if is_status => println!("No active focus session."),
        None => println!("Request completed."),
    }
    Ok(())
}

fn split_entries(entries: Vec<String>, kind: &str) -> Result<Vec<String>, String> {
    let entries = entries
        .into_iter()
        .map(|entry| entry.trim().to_owned())
        .collect::<Vec<_>>();
    if entries.iter().any(String::is_empty) {
        return Err(format!("{kind} list contains an empty entry"));
    }
    Ok(entries)
}

fn send_request(socket: &PathBuf, request: Request) -> Result<Response, String> {
    let mut stream = UnixStream::connect(socket).map_err(|error| {
        format!(
            "daemon is not reachable at {} ({error}); is focus-on-work.service running?",
            socket.display()
        )
    })?;
    let payload = serde_json::to_string(&request)
        .map_err(|error| format!("could not encode request: {error}"))?;
    stream
        .write_all(payload.as_bytes())
        .map_err(|error| format!("could not contact daemon: {error}"))?;
    stream
        .write_all(b"\n")
        .map_err(|error| format!("could not contact daemon: {error}"))?;
    let mut line = String::new();
    BufReader::new(stream)
        .read_line(&mut line)
        .map_err(|error| format!("daemon did not respond: {error}"))?;
    serde_json::from_str(&line).map_err(|error| format!("daemon returned invalid JSON: {error}"))
}

fn print_session(session: &Session) {
    let remaining = session.end_unix.saturating_sub(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
    );
    println!("Active session");
    println!("Remaining: {}", format_remaining(remaining));
    println!(
        "Sites: {}",
        format_list(session.sites.iter().map(|site| site.as_str()))
    );
    println!(
        "Apps: {}",
        format_list(session.apps.iter().map(String::as_str))
    );
}

fn format_remaining(seconds: u64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    if hours > 0 {
        format!("{hours}h {minutes}m")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

fn format_list<'a>(items: impl Iterator<Item = &'a str>) -> String {
    let items = items.collect::<Vec<_>>();
    if items.is_empty() {
        "none".into()
    } else {
        items.join(", ")
    }
}
