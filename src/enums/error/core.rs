use std::io::{self, ErrorKind};

use crate::enums::error::{mismatch::Mismatch, name::NameError};

/// Error returned by every fallible operation of the crate
///
/// [`Io`](Error::Io), [`Serde`](Error::Serde), [`Regex`](Error::Regex), [`Utf8`](Error::Utf8)
/// and [`Name`](Error::Name) display the wrapped error unchanged. Only
/// [`StateRegistry`](crate::StateRegistry) calls report a missing state file as
/// [`Error::NotFound`]; building an iterator whose state was never saved starts fresh instead
///
/// # Examples
///
/// ```
/// use preader::{Config, Error, PReader};
///
/// let dir = tempfile::tempdir()?;
///
/// let config = Config {
///     state_dir: dir.path().join("states"),
///     ..Config::default()
/// };
/// let reader = PReader::from(config);
///
/// let result = reader.states().load("job-1");
///
/// assert!(matches!(result, Err(Error::NotFound(name)) if name == "job-1"));
/// # Ok::<(), preader::Error>(())
/// ```
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// An I/O operation on a tracked file or on the state directory failed
    #[error(transparent)]
    Io(#[from] io::Error),
    /// Serializing a state to JSON failed, for example because its file path is not valid UTF-8
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    /// The [`search`](crate::StateRegistry::search) pattern is not a valid regular expression
    #[error(transparent)]
    Regex(#[from] regex::Error),
    /// A line read by a [`LineIterator`](crate::LineIterator) is not valid UTF-8
    #[error(transparent)]
    Utf8(#[from] std::str::Utf8Error),
    /// The state name breaks one of the rules described in [`NameError`]
    #[error(transparent)]
    Name(#[from] NameError),
    /// The state does not match its file, its state file or the iterator saving it
    #[error("state '{name}': {kind}")]
    Mismatch {
        /// Name of the state the error concerns
        name: String,
        /// What did not match
        kind: Mismatch,
    },
    /// The state file does not parse as a [`State`](crate::State)
    #[error("state '{name}': state file is not valid ({error})")]
    Corrupt {
        /// Name of the state the error concerns
        name: String,
        /// Error the JSON parser reported
        error: serde_json::Error,
    },
    /// A part of the state name is a symlink or a case or Unicode alias of another entry
    #[error("state '{name}': name is an alias of another entry (symlink, case or Unicode)")]
    Alias {
        /// Name of the state the error concerns
        name: String,
    },
    /// No state file is saved under the given name
    #[error("state '{0}' not found")]
    NotFound(String),
    /// The [`IteratorOptions`] [`start`] is greater than its [`end`]
    ///
    /// [`IteratorOptions`]: crate::IteratorOptions
    /// [`start`]: crate::IteratorOptions::start
    /// [`end`]: crate::IteratorOptions::end
    #[error("start must not be greater than end")]
    InvalidRange,
    /// The file's modification time cannot be represented as a UTC date and time
    #[error("file mtime is out of range")]
    InvalidMtime,
}

impl From<walkdir::Error> for Error {
    /// Converts an error from walking the state directory into [`Error::Io`]
    fn from(error: walkdir::Error) -> Self {
        Self::Io(error.into())
    }
}

impl Error {
    /// Creates an [`Error::Alias`] for the state `name`
    pub(crate) fn alias(name: &str) -> Self {
        Self::Alias {
            name: name.to_owned(),
        }
    }

    /// Creates an [`Error::Mismatch`] of `kind` for the state `name`
    pub(crate) fn mismatch(name: &str, kind: Mismatch) -> Self {
        Self::Mismatch {
            name: name.to_owned(),
            kind,
        }
    }

    /// Returns a `map_err` closure that wraps a parse error into [`Error::Corrupt`] for `name`
    pub(crate) fn corrupt(name: &str) -> impl FnOnce(serde_json::Error) -> Self {
        move |error| Self::Corrupt {
            name: name.to_owned(),
            error,
        }
    }

    /// Returns a `map_err` closure that maps an I/O `NotFound` to [`Error::NotFound`] for `name`
    ///
    /// Every other I/O error becomes [`Error::Io`] unchanged
    pub(crate) fn not_found(name: &str) -> impl FnOnce(io::Error) -> Self {
        move |error| match error.kind() {
            ErrorKind::NotFound => Self::NotFound(name.to_owned()),
            _ => error.into(),
        }
    }

    /// Turns [`Mismatch::Checksum`] of a passed [`State`](crate::State) into [`Mismatch::Unsealed`]
    ///
    /// A passed state fails its checksum because it has unsaved progress or was edited in memory,
    /// not because its state file changed; every other error is returned unchanged
    pub(crate) fn unsealed(error: Self) -> Self {
        match error {
            Self::Mismatch {
                name,
                kind: Mismatch::Checksum,
            } => Self::Mismatch {
                name,
                kind: Mismatch::Unsealed,
            },
            error => error,
        }
    }
}
