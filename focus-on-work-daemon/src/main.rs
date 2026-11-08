use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use focus_on_work_core::duration;
use focus_on_work_core::hosts::{self, Domain};
use focus_on_work_core::process;
use focus_on_work_core::session::{self, Session};
use serde::{Deserialize, Serialize};
use signal_hook::consts::{SIGINT, SIGTERM};
use signal_hook::flag;
use tracing::{info, warn};

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Serialize)]
struct Response {
    ok: bool,
    error: Option<String>,
    session: Option<Session>,
}

#[derive(Clone)]
struct Config {
    hosts: PathBuf,
    state: PathBuf,
    socket: PathBuf,
}

impl Config {
    fn from_env() -> Self {
        Self {
            hosts: env::var_os("FOW_HOSTS_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/etc/hosts")),
            state: env::var_os("FOW_STATE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/var/lib/focus-on-work/state.json")),
            socket: env::var_os("FOW_SOCKET_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/run/focus-on-work.sock")),
        }
    }

    fn apply_args(mut self) -> Self {
        let args = env::args().skip(1).collect::<Vec<_>>();
        let mut index = 0;
        while index < args.len() {
            let value = match args[index].as_str() {
                "--hosts" | "--state" | "--socket" => args.get(index + 1),
                _ => None,
            };
            if let Some(value) = value {
                match args[index].as_str() {
                    "--hosts" => self.hosts = value.into(),
                    "--state" => self.state = value.into(),
                    "--socket" => self.socket = value.into(),
                    _ => unreachable!(),
                }
                index += 2;
            } else {
                index += 1;
            }
        }
        self
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let config = Config::from_env().apply_args();
    if let Some(parent) = config.socket.parent() {
        fs::create_dir_all(parent)?;
    }
    let _ = fs::remove_file(&config.socket);
    let listener = UnixListener::bind(&config.socket)?;
    listener.set_nonblocking(true)?;
    let stopping = Arc::new(AtomicBool::new(false));
    flag::register(SIGTERM, Arc::clone(&stopping))?;
    flag::register(SIGINT, Arc::clone(&stopping))?;
    let current = match session::load(&config.state) {
        Ok(session) => session,
        Err(error) => {
            warn!(%error, "ignoring invalid state file");
            if let Err(remove_error) = hosts::remove_block_if_complete(&config.hosts) {
                warn!(%remove_error, "could not remove leftover hosts block");
            }
            None
        }
    };
    let current = if current
        .as_ref()
        .is_some_and(|s| !s.is_active(session::now_unix()))
    {
        if let Err(error) = hosts::remove_block(&config.hosts) {
            warn!(%error, "could not remove expired hosts block");
        }
        if let Err(error) = session::clear(&config.state) {
            warn!(%error, "could not clear expired state");
        }
        None
    } else {
        current
    };
    let current = Arc::new(Mutex::new(current));
    let accept_config = config.clone();
    let accept_current = Arc::clone(&current);
    let accept_stopping = Arc::clone(&stopping);
    thread::spawn(move || {
        while !accept_stopping.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, _)) => {
                    let config = accept_config.clone();
                    let current = Arc::clone(&accept_current);
                    thread::spawn(move || {
                        if let Err(error) = handle_client(stream, &config, &current) {
                            warn!(%error, "client request failed");
                        }
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(50));
                }
                Err(error) => warn!(%error, "socket accept failed"),
            }
        }
    });
    info!("daemon started");
    while !stopping.load(Ordering::Relaxed) {
        let mut state = current
            .lock()
            .map_err(|_| io::Error::other("state lock poisoned"))?;
        if let Some(active) = state.as_ref() {
            if !hosts::is_block_present(&config.hosts, &active.sites).unwrap_or(false)
                && let Err(error) = hosts::repair_block(&config.hosts, &active.sites)
            {
                warn!(%error, "could not repair managed hosts block");
            }
            process::terminate_matching(&active.apps, Some(std::process::id()));
            if !active.is_active(session::now_unix()) {
                if let Err(error) = hosts::remove_block(&config.hosts) {
                    warn!(%error, "could not remove expired hosts block");
                }
                if let Err(error) = session::clear(&config.state) {
                    warn!(%error, "could not clear expired state");
                }
                *state = None;
                info!("session expired");
            }
        }
        drop(state);
        thread::sleep(Duration::from_secs(2));
    }
    info!("daemon stopping; active hosts block remains in place");
    let _ = fs::remove_file(&config.socket);
    Ok(())
}

fn handle_client(
    mut stream: UnixStream,
    config: &Config,
    current: &Arc<Mutex<Option<Session>>>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut line = String::new();
    BufReader::new(stream.try_clone()?).read_line(&mut line)?;
    let mut current = current
        .lock()
        .map_err(|_| io::Error::other("state lock poisoned"))?;
    let response = match serde_json::from_str::<Request>(&line) {
        Ok(request) => handle_request(request, config, &mut current),
        Err(error) => Response {
            ok: false,
            error: Some(format!("invalid request: {error}")),
            session: None,
        },
    };
    serde_json::to_writer(&mut stream, &response)?;
    stream.write_all(b"\n")?;
    Ok(())
}

fn handle_request(request: Request, config: &Config, current: &mut Option<Session>) -> Response {
    match request {
        Request::Status => Response {
            ok: true,
            error: None,
            session: current.clone(),
        },
        Request::Stop => {
            if current.is_some() {
                return Response {
                    ok: false,
                    error: Some("session is active; stop is refused".into()),
                    session: current.clone(),
                };
            }
            Response {
                ok: true,
                error: None,
                session: None,
            }
        }
        Request::Start {
            duration: duration_text,
            sites,
            apps,
        } => {
            if let Some(active) = current {
                return Response {
                    ok: false,
                    error: Some("session is already active".into()),
                    session: Some(active.clone()),
                };
            }
            let seconds = match duration::parse(&duration_text) {
                Ok(value) => value.as_secs(),
                Err(error) => {
                    return Response {
                        ok: false,
                        error: Some(error.to_string()),
                        session: None,
                    };
                }
            };
            let domains = match sites
                .iter()
                .map(|site| hosts::validate_domain(site))
                .collect::<Result<Vec<Domain>, _>>()
            {
                Ok(domains) => domains,
                Err(error) => {
                    return Response {
                        ok: false,
                        error: Some(error.to_string()),
                        session: None,
                    };
                }
            };
            if apps
                .iter()
                .any(|app| app.is_empty() || app.len() > 255 || app.chars().any(|c| c.is_control()))
            {
                return Response {
                    ok: false,
                    error: Some("invalid app entry".into()),
                    session: None,
                };
            }
            let session = Session {
                end_unix: session::now_unix() + seconds,
                sites: domains,
                apps,
            };
            if let Err(error) = hosts::apply_block(&config.hosts, &session.sites).and_then(|_| {
                session::save(&config.state, &session)
                    .map_err(|error| hosts::Error::Io(io::Error::other(error)))
            }) {
                return Response {
                    ok: false,
                    error: Some(error.to_string()),
                    session: None,
                };
            }
            *current = Some(session.clone());
            Response {
                ok: true,
                error: None,
                session: Some(session),
            }
        }
    }
}
