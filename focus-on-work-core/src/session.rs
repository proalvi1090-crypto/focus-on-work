use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::hosts::Domain;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub end_unix: u64,
    pub sites: Vec<Domain>,
    pub apps: Vec<String>,
}

impl Session {
    pub fn is_active(&self, now: u64) -> bool {
        now < self.end_unix
    }

    pub fn remaining(&self, now: u64) -> u64 {
        self.end_unix.saturating_sub(now)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error("invalid session JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid session domain: {0}")]
    InvalidDomain(String),
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn load(path: &Path) -> Result<Option<Session>, Error> {
    match fs::read(path) {
        Ok(bytes) => {
            let session: Session = serde_json::from_slice(&bytes)?;
            for site in &session.sites {
                crate::hosts::validate_domain(site.as_str())
                    .map_err(|error| Error::InvalidDomain(error.to_string()))?;
            }
            Ok(Some(session))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

pub fn save(path: &Path, session: &Session) -> Result<(), Error> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "state path has no parent"))?;
    fs::create_dir_all(parent)?;
    let temp = PathBuf::from(format!("{}.tmp", path.display()));
    let bytes = serde_json::to_vec_pretty(session)?;
    fs::write(&temp, bytes)?;
    fs::rename(temp, path)?;
    Ok(())
}

pub fn clear(path: &Path) -> Result<(), Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn session_time_and_json_round_trip() {
        let session = Session {
            end_unix: 100,
            sites: vec![crate::hosts::validate_domain("example.com").unwrap()],
            apps: vec!["editor".into()],
        };
        assert!(session.is_active(99));
        assert_eq!(session.remaining(150), 0);
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        save(&path, &session).unwrap();
        assert_eq!(load(&path).unwrap(), Some(session));
    }

    #[test]
    fn missing_or_corrupt_state_is_safe_to_handle() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("state.json");
        assert_eq!(load(&path).unwrap(), None);
        fs::write(&path, "not json").unwrap();
        assert!(load(&path).is_err());
    }
}
