use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    NameError,
    constants::{NAME_SEPARATOR, STATE_DIR_NAME},
    validators::validate_name,
};

pub fn default_state_dir() -> PathBuf {
    scoped_join(&dirs::cache_dir().unwrap_or_default(), STATE_DIR_NAME).unwrap()
}

pub fn scoped_join(root: &Path, name: &str) -> Result<PathBuf, NameError> {
    Ok(root.join(validate_name(name)?.split(NAME_SEPARATOR).collect::<PathBuf>()))
}

pub fn resolves_in_place(root: &Path, path: &Path) -> bool {
    let (Ok(canonical), Ok(relative)) = (fs::canonicalize(root), path.strip_prefix(root)) else {
        return true;
    };
    let scoped = canonical.join(relative);

    scoped
        .ancestors()
        .find(|ancestor| ancestor.symlink_metadata().is_ok())
        .is_none_or(|existing| fs::canonicalize(existing).is_ok_and(|real| real == existing))
}
