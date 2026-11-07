use std::time::Duration;

use thiserror::Error;

const MAX_SECONDS: u64 = 365 * 24 * 60 * 60;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("duration must contain a positive value")]
    Invalid,
    #[error("duration is too large")]
    TooLarge,
}

pub fn parse(input: &str) -> Result<Duration, Error> {
    if input.is_empty() {
        return Err(Error::Invalid);
    }
    let mut total = 0u64;
    let mut number = 0u64;
    let mut saw_unit = false;
    let mut digits = false;
    for byte in input.bytes() {
        if byte.is_ascii_digit() {
            digits = true;
            number = number
                .checked_mul(10)
                .and_then(|n| n.checked_add((byte - b'0') as u64))
                .ok_or(Error::TooLarge)?;
        } else {
            let multiplier = match byte {
                b'm' => 60,
                b'h' => 60 * 60,
                b'd' => 24 * 60 * 60,
                _ => return Err(Error::Invalid),
            };
            if !digits || number == 0 {
                return Err(Error::Invalid);
            }
            total = total
                .checked_add(number.checked_mul(multiplier).ok_or(Error::TooLarge)?)
                .ok_or(Error::TooLarge)?;
            number = 0;
            digits = false;
            saw_unit = true;
        }
    }
    if digits || !saw_unit || total == 0 {
        return Err(Error::Invalid);
    }
    if total > MAX_SECONDS {
        return Err(Error::TooLarge);
    }
    Ok(Duration::from_secs(total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_compound_durations() {
        assert_eq!(parse("90m").unwrap().as_secs(), 5400);
        assert_eq!(parse("2h").unwrap().as_secs(), 7200);
        assert_eq!(parse("1h30m").unwrap().as_secs(), 5400);
    }

    #[test]
    fn rejects_invalid_and_absurd_durations() {
        for value in ["", "0m", "1", "1x", "1hm", "1m2", "999999999d"] {
            assert!(parse(value).is_err(), "{value}");
        }
    }
}
