/// Error for a state name that cannot be used as a state file path
///
/// A state name is a path relative to [`Config::state_dir`](crate::Config::state_dir) with `/`
/// as the separator on every platform; it is measured in UTF-8 bytes and never normalized. The
/// checks run in order: [`Empty`](NameError::Empty), then [`Escapes`](NameError::Escapes), then
/// [`Invalid`](NameError::Invalid). It reaches callers wrapped in [`Error::Name`]
///
/// A name is invalid when it is longer than 512 bytes, its last part is longer than 219 bytes, a
/// directory part is longer than 255 bytes or ends in `.` or a space, or any part is empty or
/// `.`. Every part may contain only letters, marks, numbers, punctuation, symbols and space
/// separators, but none of `\ : * ? " < > |`. No part may be a Windows device name (`CON`, `PRN`,
/// `AUX`, `NUL`, `CONIN$`, `CONOUT$`, `COM1` to `COM9`, `LPT1` to `LPT9`, and `COM` or `LPT`
/// followed by `¹`, `²` or `³`) in any case, with any trailing spaces or extension. The Windows
/// rules apply on every platform, so a name valid on one system is valid on all
///
/// # Examples
///
/// ```
/// use preader::{Error, NameError, PReader};
///
/// let result = PReader::default().states().path("../job-1");
///
/// assert!(matches!(result, Err(Error::Name(NameError::Escapes))));
/// ```
///
/// [`Error::Name`]: crate::Error::Name
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum NameError {
    /// The name is empty
    #[error("state name must not be empty")]
    Empty,
    /// The name starts with `/` or has a `..` part, so it escapes the state directory
    #[error("state name escapes the state directory")]
    Escapes,
    /// The name is too long or has a part that is not a portable file name
    #[error("state name is not valid")]
    Invalid,
}
