use std::fmt;

/// Errors from the TOTP implementation.
#[derive(Debug)]
pub enum Error {
    InvalidSeed,
    Bounds,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidSeed => write!(f, "invalid seed"),
            Error::Bounds => write!(f, "argument out-of-bounds"),
        }
    }
}

impl From<Error> for std::io::Error {
    fn from(e: Error) -> std::io::Error {
        std::io::Error::other(e)
    }
}
