use crate::{PathError, constants::NAME_SEPARATOR, macros::ensure};

// Separator between a file stem and its extensions, used to find a device name in "nul.txt"
const EXTENSION_SEPARATOR: char = '.';
// Name part that means the current directory, rejected so a name has a single spelling
const CURRENT_DIR: &str = ".";
// Name part that means the parent directory, rejected so a name cannot leave the state directory
const PARENT_DIR: &str = "..";
// Characters Windows does not allow in a file name, rejected on every platform
const RESERVED_CHARS: [char; 8] = ['\\', ':', '*', '?', '"', '<', '>', '|'];
// Characters Win32 strips from the end of a directory name, which would alias another directory
const TRIMMED_CHARS: [char; 2] = ['.', ' '];
// Windows device names, reserved with any extension; see learn.microsoft.com "Naming Files"
const DEVICE_NAMES: [&str; 30] = [
    "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "COM1", "COM2", "COM3", "COM4", "COM5",
    "COM6", "COM7", "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5",
    "LPT6", "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³",
];

pub fn validate_name(name: &str) -> Result<&str, PathError> {
    ensure!(!name.is_empty(), PathError::Empty);
    ensure!(!escapes(name), PathError::Escapes(name.to_owned()));
    ensure!(is_portable(name), PathError::Invalid(name.to_owned()));

    Ok(name)
}

fn escapes(name: &str) -> bool {
    name.starts_with(NAME_SEPARATOR) || name.split(NAME_SEPARATOR).any(|part| part == PARENT_DIR)
}

fn is_portable(name: &str) -> bool {
    let mut parts = name.rsplit(NAME_SEPARATOR);
    let file = parts.next().unwrap_or(name);

    is_portable_part(file)
        && parts.all(|dir| is_portable_part(dir) && !dir.ends_with(TRIMMED_CHARS))
}

fn is_portable_part(part: &str) -> bool {
    !matches!(part, "" | CURRENT_DIR)
        && !part.contains(|character: char| {
            character.is_control() || RESERVED_CHARS.contains(&character)
        })
        && !is_device(part)
}

fn is_device(part: &str) -> bool {
    DEVICE_NAMES.contains(
        &part
            .split(EXTENSION_SEPARATOR)
            .next()
            .unwrap_or(part)
            .trim_end()
            .to_ascii_uppercase()
            .as_str(),
    )
}
