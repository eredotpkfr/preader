use std::sync::LazyLock;

use regex::Regex;

use crate::{
    NameError,
    constants::{CURRENT_DIR, NAME_SEPARATOR, STATE_FILE_EXTENSION, TMP_FILE_EXTENSION},
    macros::ensure,
};

/// Name part that means the parent directory, rejected so a name cannot leave the state directory
const PARENT_DIR: &str = "..";
/// Characters Win32 strips from the end of a directory name, which would alias another directory
const TRIMMED_CHARS: [char; 2] = ['.', ' '];
/// Longest file name on disk in UTF-8 bytes, the strictest of Linux, APFS and NTFS
const MAX_PART_BYTES: usize = 255;
/// Longest last part in UTF-8 bytes, so the temporary file's `.{nanos}.state.json.tmp` still fits
const MAX_FILE_BYTES: usize =
    MAX_PART_BYTES - (1 + 20 + 1 + STATE_FILE_EXTENSION.len() + 1 + TMP_FILE_EXTENSION.len());
/// Longest name in UTF-8 bytes, leaving room for [`state_dir`] under macOS's 1023-byte path limit
///
/// [`state_dir`]: crate::Config::state_dir
const MAX_NAME_BYTES: usize = 512;
/// Characters Windows does not allow in a file name, rejected on every platform
const RESERVED_CHARS: [char; 8] = ['\\', ':', '*', '?', '"', '<', '>', '|'];
/// Windows device names, reserved with trailing spaces and any extension, per [Naming Files]
///
/// [Naming Files]: https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file
const DEVICE_NAMES: [&str; 30] = [
    "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "COM1", "COM2", "COM3", "COM4", "COM5",
    "COM6", "COM7", "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
    "LPT6", "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³",
];

/// Matches a name part of letters, marks, numbers, punctuation, symbols and space separators
///
/// [`RESERVED_CHARS`] are excluded. As an allowlist it rejects control, format (bidi, zero-width),
/// private-use and unassigned code points
static PORTABLE_PART: LazyLock<Regex> = LazyLock::new(|| {
    let reserved = regex::escape(&String::from_iter(RESERVED_CHARS));

    Regex::new(&format!(r"^[\pL\pM\pN\pP\pS\p{{Zs}}--[{reserved}]]+$")).unwrap()
});
/// Matches a part that is one of [`DEVICE_NAMES`] in any case, with trailing spaces or extension
static DEVICE_NAME: LazyLock<Regex> = LazyLock::new(|| {
    let names = DEVICE_NAMES.map(regex::escape).join("|");

    Regex::new(&format!(r"(?i)^(?:{names}) *(?:\..*)?$")).unwrap()
});

/// Returns `name` unchanged when it is a valid state name
///
/// The rules are listed on [`NameError`] and are the same on every platform
///
/// # Errors
///
/// Returns [`NameError::Empty`] when `name` is empty, [`NameError::Escapes`] when it starts
/// with `/` or has a `..` part, and [`NameError::Invalid`] when it breaks any other rule
///
/// # Examples
///
/// ```
/// use preader::{NameError, validate_name};
///
/// let result = validate_name("logs/con.txt");
///
/// assert_eq!(result, Err(NameError::Invalid));
/// ```
pub fn validate_name(name: &str) -> Result<&str, NameError> {
    ensure!(!name.is_empty(), NameError::Empty);
    ensure!(!escapes(name), NameError::Escapes);
    ensure!(is_portable(name), NameError::Invalid);

    Ok(name)
}

/// Returns whether `name` is absolute or has a [`PARENT_DIR`] part
fn escapes(name: &str) -> bool {
    name.starts_with(NAME_SEPARATOR) || name.split(NAME_SEPARATOR).any(|part| part == PARENT_DIR)
}

/// Returns whether `name` fits [`MAX_NAME_BYTES`] and every part is a portable file or directory
///
/// The last part is the file and is limited to [`MAX_FILE_BYTES`]. Every other part is a
/// directory, limited to [`MAX_PART_BYTES`] and not ending in one of [`TRIMMED_CHARS`]
fn is_portable(name: &str) -> bool {
    let mut parts = name.rsplit(NAME_SEPARATOR);
    let is_file = |file| is_portable_part(file, MAX_FILE_BYTES);
    let is_dir = |dir: &str| is_portable_part(dir, MAX_PART_BYTES) && !dir.ends_with(TRIMMED_CHARS);

    name.len() <= MAX_NAME_BYTES && parts.next().is_some_and(is_file) && parts.all(is_dir)
}

/// Returns whether `part` is a portable name of at most `max_bytes` bytes
///
/// It must match [`PORTABLE_PART`] and be neither [`CURRENT_DIR`] nor a [`DEVICE_NAME`] match
fn is_portable_part(part: &str, max_bytes: usize) -> bool {
    part.len() <= max_bytes
        && part != CURRENT_DIR
        && PORTABLE_PART.is_match(part)
        && !DEVICE_NAME.is_match(part)
}
