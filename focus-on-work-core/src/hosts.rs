//! Safe, atomic editing of the managed section in a hosts file.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

const BEGIN_MARKER: &str = "# BEGIN focus-on-work";
const END_MARKER: &str = "# END focus-on-work";

/// A domain name that passed the hosts-file validator.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Domain(String);

impl Domain {
    /// Returns the validated domain as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Errors returned by hosts-file operations.
#[derive(Debug, Error)]
pub enum Error {
    /// The domain contains a character or label that is not allowed.
    #[error("invalid domain: {0}")]
    InvalidDomain(String),
    /// The marker section is incomplete or appears more than once.
    #[error("corrupted focus-on-work markers: {0}")]
    CorruptedMarkers(&'static str),
    /// A filesystem operation failed.
    #[error(transparent)]
    Io(#[from] io::Error),
    /// The hosts file is not valid UTF-8.
    #[error("hosts file is not valid UTF-8")]
    InvalidUtf8,
    /// The path cannot be used as a regular file path.
    #[error("invalid hosts file path")]
    InvalidPath,
}

/// Validates a domain and returns a safe domain newtype.
pub fn validate_domain(value: &str) -> Result<Domain, Error> {
    if value.is_empty() || value.len() > 253 || value.trim() != value {
        return Err(Error::InvalidDomain(value.to_owned()));
    }

    for label in value.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(Error::InvalidDomain(value.to_owned()));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(Error::InvalidDomain(value.to_owned()));
        }
        if !label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(Error::InvalidDomain(value.to_owned()));
        }
    }

    Ok(Domain(value.to_ascii_lowercase()))
}

/// Builds the managed marker block for the supplied domains.
pub fn build_block(domains: &[Domain]) -> String {
    let mut lines = vec![BEGIN_MARKER.to_owned()];
    for domain in domains {
        lines.push(format!("127.0.0.1 {}", domain.as_str()));
        lines.push(format!("127.0.0.1 www.{}", domain.as_str()));
        lines.push(format!("::1 {}", domain.as_str()));
        lines.push(format!("::1 www.{}", domain.as_str()));
    }
    lines.push(END_MARKER.to_owned());
    lines.join("\n")
}

/// Applies the managed block, backing up the old file and atomically replacing it.
pub fn apply_block(path: &Path, domains: &[Domain]) -> Result<(), Error> {
    let original = read_file(path)?;
    let text = String::from_utf8(original.clone()).map_err(|_| Error::InvalidUtf8)?;
    let ending = line_ending(&text);
    let block = build_block(domains).replace('\n', ending);
    let (start, end) = marker_bounds(&text)?;
    let replacement = if let Some((start, end)) = start.zip(end) {
        replace_section(&text, start, end, &block, ending)
    } else {
        append_section(&text, &block, ending)
    };

    let has_existing_block = start.is_some() && end.is_some();
    if !has_existing_block {
        let backup = backup_path(path);
        let refresh_backup = match fs::read(&backup) {
            Ok(contents) => contents != original,
            Err(error) if error.kind() == io::ErrorKind::NotFound => true,
            Err(error) => return Err(error.into()),
        };
        if refresh_backup {
            fs::copy(path, backup)?;
        }
    }
    atomic_write(path, replacement.as_bytes())?;
    Ok(())
}

/// Removes the managed block. It is safe to call when no block exists.
pub fn remove_block(path: &Path) -> Result<(), Error> {
    let bytes = read_file(path)?;
    let text = String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
    let (start, end) = marker_bounds(&text)?;
    if let Some((start, end)) = start.zip(end) {
        let updated = remove_section(&text, start, end);
        atomic_write(path, updated.as_bytes())?;
    }
    Ok(())
}

/// Removes a complete managed block, leaving malformed files untouched.
pub fn remove_block_if_complete(path: &Path) -> Result<bool, Error> {
    let bytes = read_file(path)?;
    let text = String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
    let (start, end) = marker_bounds(&text)?;
    if start.is_some() && end.is_some() {
        remove_block(path)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

/// Rewrites the managed block even when old marker lines are malformed.
pub fn repair_block(path: &Path, domains: &[Domain]) -> Result<(), Error> {
    let bytes = read_file(path)?;
    let text = String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
    match marker_bounds(&text) {
        Ok(_) => apply_block(path, domains),
        Err(_) => {
            let lines = text
                .lines()
                .filter(|line| *line != BEGIN_MARKER && *line != END_MARKER)
                .collect::<Vec<_>>()
                .join("\n");
            let ending = line_ending(&text);
            let block = build_block(domains).replace('\n', ending);
            let replacement = append_section(&lines, &block, ending);
            atomic_write(path, replacement.as_bytes())
        }
    }
}

/// Reports whether the managed block exactly matches the supplied domains.
pub fn is_block_present(path: &Path, domains: &[Domain]) -> Result<bool, Error> {
    let bytes = read_file(path)?;
    let text = String::from_utf8(bytes).map_err(|_| Error::InvalidUtf8)?;
    let (start, end) = marker_bounds(&text)?;
    let Some((start, end)) = start.zip(end) else {
        return Ok(false);
    };
    let actual_end = line_end(&text, end);
    let actual = text[start..actual_end].trim_end_matches(['\r', '\n']);
    let expected = build_block(domains);
    Ok(actual == expected || actual == expected.replace('\n', "\r\n"))
}

fn read_file(path: &Path) -> Result<Vec<u8>, Error> {
    Ok(fs::read(path)?)
}

fn backup_path(path: &Path) -> PathBuf {
    let mut backup = path.as_os_str().to_os_string();
    backup.push(".bak");
    PathBuf::from(backup)
}

fn line_ending(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

fn marker_bounds(text: &str) -> Result<(Option<usize>, Option<usize>), Error> {
    let mut begin = None;
    let mut end = None;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let content = line.trim_end_matches(['\n', '\r']);
        if content == BEGIN_MARKER {
            if begin.replace(offset).is_some() {
                return Err(Error::CorruptedMarkers("duplicate start marker"));
            }
        } else if content == END_MARKER && end.replace(offset).is_some() {
            return Err(Error::CorruptedMarkers("duplicate end marker"));
        }
        offset += line.len();
    }
    match (begin, end) {
        (Some(_), None) | (None, Some(_)) => {
            Err(Error::CorruptedMarkers("only one marker is present"))
        }
        (Some(start), Some(end)) if start > end => {
            Err(Error::CorruptedMarkers("end marker precedes start marker"))
        }
        _ => Ok((begin, end)),
    }
}

fn line_end(text: &str, marker_start: usize) -> usize {
    text[marker_start..]
        .find('\n')
        .map_or(text.len(), |index| marker_start + index + 1)
}

fn replace_section(text: &str, start: usize, end: usize, block: &str, ending: &str) -> String {
    let end = line_end(text, end);
    let mut result = String::with_capacity(text.len());
    result.push_str(&text[..start]);
    result.push_str(block);
    if end < text.len() || text.ends_with(ending) {
        result.push_str(ending);
    }
    result.push_str(&text[end..]);
    result
}

fn append_section(text: &str, block: &str, ending: &str) -> String {
    let mut result = String::with_capacity(text.len() + block.len() + ending.len());
    result.push_str(text);
    if !text.is_empty() && !text.ends_with(['\n', '\r']) {
        result.push_str(ending);
    }
    result.push_str(block);
    result
}

fn remove_section(text: &str, start: usize, end: usize) -> String {
    let end = line_end(text, end);
    let mut result = String::with_capacity(text.len());
    result.push_str(&text[..start]);
    result.push_str(&text[end..]);
    result
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), Error> {
    if path.parent().is_none() {
        return Err(Error::InvalidPath);
    }
    let temp = tempfile_path(path);
    let mut file = fs::File::create(&temp)?;
    file.write_all(contents)?;
    file.sync_all()?;
    if let Ok(metadata) = fs::metadata(path) {
        fs::set_permissions(&temp, metadata.permissions())?;
    }
    fs::rename(&temp, path)?;
    Ok(())
}

fn tempfile_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new(""));
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    parent.join(format!(".{name}.tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn domain(value: &str) -> Domain {
        validate_domain(value).expect("test domain is valid")
    }

    #[test]
    fn validates_domains_and_rejects_injection() {
        assert!(validate_domain("example.com").is_ok());
        for invalid in [
            "",
            " example.com",
            "example.com ",
            "a\n127.0.0.1 evil",
            "a#b.com",
            "a..com",
            "-a.com",
            "a-.com",
        ] {
            assert!(validate_domain(invalid).is_err(), "{invalid:?}");
        }
    }

    #[test]
    fn builds_ipv4_and_ipv6_entries() {
        let block = build_block(&[domain("example.com")]);
        assert!(block.contains("127.0.0.1 example.com"));
        assert!(block.contains("127.0.0.1 www.example.com"));
        assert!(block.contains("::1 example.com"));
        assert!(block.contains("::1 www.example.com"));
    }

    #[test]
    fn apply_preserves_unrelated_lines_and_is_idempotent() {
        let directory = tempdir().expect("tempdir");
        let path = directory.path().join("hosts");
        fs::write(&path, "127.0.0.1 localhost\ncustom line").expect("write");
        let domains = [domain("example.com")];
        apply_block(&path, &domains).expect("apply");
        let first = fs::read_to_string(&path).expect("read");
        assert_eq!(
            "127.0.0.1 localhost\ncustom line",
            fs::read_to_string(path.with_extension("bak")).expect("backup")
        );
        apply_block(&path, &domains).expect("apply again");
        assert_eq!(first, fs::read_to_string(&path).expect("read again"));
        assert!(first.starts_with("127.0.0.1 localhost\ncustom line\n"));
        assert!(is_block_present(&path, &domains).expect("presence"));
    }

    #[test]
    fn apply_preserves_missing_trailing_newline() {
        let directory = tempdir().expect("tempdir");
        let path = directory.path().join("hosts");
        fs::write(&path, "before").expect("write");

        apply_block(&path, &[domain("example.com")]).expect("apply");
        let result = fs::read_to_string(&path).expect("read");

        assert!(result.starts_with("before\n# BEGIN focus-on-work\n"));
        assert!(result.ends_with("# END focus-on-work"));
    }

    #[test]
    fn remove_is_idempotent_and_preserves_no_trailing_newline() {
        let directory = tempdir().expect("tempdir");
        let path = directory.path().join("hosts");
        fs::write(&path, "before\n").expect("write");
        let domains = [domain("example.com")];
        apply_block(&path, &domains).expect("apply");
        remove_block(&path).expect("remove");
        assert_eq!("before\n", fs::read_to_string(&path).expect("read"));
        remove_block(&path).expect("remove again");
        assert_eq!("before\n", fs::read_to_string(&path).expect("read again"));
    }

    #[test]
    fn replaces_duplicate_marker_section_without_duplication() {
        let directory = tempdir().expect("tempdir");
        let path = directory.path().join("hosts");
        fs::write(
            &path,
            "before\n# BEGIN focus-on-work\nold\n# END focus-on-work\nafter",
        )
        .expect("write");
        apply_block(&path, &[domain("new.example")]).expect("replace");
        let result = fs::read_to_string(&path).expect("read");
        assert_eq!(result.matches(BEGIN_MARKER).count(), 1);
        assert!(result.contains("new.example"));
        assert!(result.contains("before\n"));
        assert!(result.ends_with("after"));
    }

    #[test]
    fn rejects_incomplete_and_duplicate_markers() {
        let directory = tempdir().expect("tempdir");
        let path = directory.path().join("hosts");
        fs::write(&path, "# BEGIN focus-on-work\n").expect("write");
        assert!(matches!(
            apply_block(&path, &[]),
            Err(Error::CorruptedMarkers(_))
        ));
        fs::write(&path, "# BEGIN focus-on-work\n# END focus-on-work\n# BEGIN focus-on-work\n# END focus-on-work").expect("write");
        assert!(matches!(
            remove_block(&path),
            Err(Error::CorruptedMarkers(_))
        ));
    }

    #[test]
    fn empty_domains_still_create_markers() {
        let directory = tempdir().expect("tempdir");
        let path = directory.path().join("hosts");
        fs::write(&path, "").expect("write");
        apply_block(&path, &[]).expect("apply");
        assert!(is_block_present(&path, &[]).expect("presence"));
    }

    #[test]
    fn repair_replaces_corrupt_marker_lines() {
        let directory = tempdir().expect("tempdir");
        let path = directory.path().join("hosts");
        fs::write(&path, "before\n# BEGIN focus-on-work\nbroken\n").expect("write");
        repair_block(&path, &[domain("example.com")]).expect("repair");
        let result = fs::read_to_string(&path).expect("read");
        assert!(result.contains("before\n"));
        assert!(is_block_present(&path, &[domain("example.com")]).expect("presence"));
    }

    #[test]
    fn applying_existing_block_does_not_replace_backup() {
        let directory = tempdir().expect("tempdir");
        let path = directory.path().join("hosts");
        fs::write(&path, "before\n").expect("write");
        apply_block(&path, &[domain("old.example")]).expect("apply");
        apply_block(&path, &[domain("new.example")]).expect("replace");
        assert_eq!(
            fs::read_to_string(path.with_extension("bak")).expect("backup"),
            "before\n"
        );
    }

    #[test]
    fn refreshes_backup_after_user_changes_unmanaged_file() {
        let directory = tempdir().expect("tempdir");
        let path = directory.path().join("hosts");
        fs::write(&path, "before\n").expect("write");
        apply_block(&path, &[domain("old.example")]).expect("apply");
        remove_block(&path).expect("remove");
        fs::write(&path, "user edit\n").expect("edit");

        apply_block(&path, &[domain("new.example")]).expect("apply again");

        assert_eq!(
            fs::read_to_string(path.with_extension("bak")).expect("backup"),
            "user edit\n"
        );
    }
}
