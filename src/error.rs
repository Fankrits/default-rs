//! The error type returned by every operation.

use std::fmt;

/// Why an operation failed.
#[derive(Debug)]
pub enum Error {
    /// The operation does not exist on this OS (for example [`crate::register`] on macOS).
    Unsupported(&'static str),
    /// The [`crate::App`] has no identifier for this OS. The name says which one to set.
    MissingIdentifier(&'static str),
    /// The [`crate::FileType`] has no value this OS needs. The name says which one to set.
    MissingFileType(&'static str),
    /// A value cannot be written safely (for example a path with a newline).
    InvalidInput(String),
    /// The OS does not know the app, so it cannot become the default (not installed, or no desktop file).
    NotInstalled(String),
    /// A helper program is not installed (`xdg-mime` on Linux).
    ToolMissing(&'static str),
    /// The OS or a helper program refused or reported a failure.
    Failed(String),
    /// A file or registry operation failed.
    Io(std::io::Error),
}

/// `Result` with this crate's [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Unsupported(what) => write!(f, "not supported on this OS: {what}"),
            Error::MissingIdentifier(which) => {
                write!(f, "the App has no {which}, which this OS needs")
            }
            Error::MissingFileType(which) => {
                write!(f, "the FileType has no {which}, which this OS needs")
            }
            Error::InvalidInput(why) => write!(f, "invalid input: {why}"),
            Error::NotInstalled(why) => write!(f, "app is not installed: {why}"),
            Error::ToolMissing(tool) => write!(f, "`{tool}` is not installed"),
            Error::Failed(why) => write!(f, "{why}"),
            Error::Io(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}
