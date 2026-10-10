use std::sync::LazyLock;

use regex::Regex;

use crate::{
    NameError,
    constants::{NAME_SEPARATOR, STATE_FILE_EXTENSION, TMP_FILE_EXTENSION},
    macros::ensure,
};

// Name part that means the current directory, rejected so a name has a single spelling
const CURRENT_DIR: &str = ".";
// Name part that means the parent directory, rejected so a name cannot leave the state directory
const PARENT_DIR: &str = "..";
// Characters Win32 strips from the end of a directory name, which would alias another directory
const TRIMMED_CHARS: [char; 2] = ['.', ' '];
// Longest file name on disk in UTF-8 bytes, the strictest of Linux, APFS and NTFS
const MAX_PART_BYTES: usize = 255;
// Longest last part; the temporary file appends ".{signed i64 nanoseconds}.state.json.tmp"
const MAX_FILE_BYTES: usize =
    MAX_PART_BYTES - (1 + 20 + STATE_FILE_EXTENSION.len() + 1 + TMP_FILE_EXTENSION.len());
// Keeps the state file under macOS's 1023-byte absolute path limit with room left for state_dir
const MAX_NAME_BYTES: usize = 512;
// Characters Windows does not allow in a file name, rejected on every platform
const RESERVED_CHARS: [char; 8] = ['\\', ':', '*', '?', '"', '<', '>', '|'];
// Windows device names, reserved with trailing spaces and any extension; see "Naming Files"
const DEVICE_NAMES: [&str; 30] = [
    "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "COM1", "COM2", "COM3", "COM4", "COM5",
    "COM6", "COM7", "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
    "LPT6", "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³",
];

// Allowlist, so control, format (bidi, zero-width), private-use and unassigned code points fail
static PORTABLE_PART: LazyLock<Regex> = LazyLock::new(|| {
    let reserved = regex::escape(&String::from_iter(RESERVED_CHARS));

    Regex::new(&format!(r"^[\pL\pM\pN\pP\pS\p{{Zs}}--[{reserved}]]+$")).unwrap()
});
static DEVICE_NAME: LazyLock<Regex> = LazyLock::new(|| {
    let names = DEVICE_NAMES.map(regex::escape).join("|");

    Regex::new(&format!(r"(?i)^(?:{names}) *(?:\..*)?$")).unwrap()
});

pub fn validate_name(name: &str) -> Result<&str, NameError> {
    ensure!(!name.is_empty(), NameError::Empty);
    ensure!(!escapes(name), NameError::Escapes);
    ensure!(is_portable(name), NameError::Invalid);

    Ok(name)
}

fn escapes(name: &str) -> bool {
    name.starts_with(NAME_SEPARATOR) || name.split(NAME_SEPARATOR).any(|part| part == PARENT_DIR)
}

fn is_portable(name: &str) -> bool {
    let mut parts = name.rsplit(NAME_SEPARATOR);
    let is_file = |file| is_portable_part(file, MAX_FILE_BYTES);
    let is_dir = |dir: &str| is_portable_part(dir, MAX_PART_BYTES) && !dir.ends_with(TRIMMED_CHARS);

    name.len() <= MAX_NAME_BYTES && parts.next().is_some_and(is_file) && parts.all(is_dir)
}

fn is_portable_part(part: &str, max_bytes: usize) -> bool {
    part.len() <= max_bytes
        && part != CURRENT_DIR
        && PORTABLE_PART.is_match(part)
        && !DEVICE_NAME.is_match(part)
}
