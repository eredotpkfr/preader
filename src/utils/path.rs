use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    NameError,
    constants::{NAME_SEPARATOR, STATE_DIR_NAME},
    validators::validate_name,
};

/// Returns the default state directory, `preader` inside the user's cache directory
///
/// When the platform has no cache directory the result is the relative path `preader`. Nothing is
/// created on disk
///
/// # Panics
///
/// Never panics in practice: the `unwrap` fails only if [`STATE_DIR_NAME`] were not a valid
/// state name
///
/// # Examples
///
/// ```
/// use preader::{STATE_DIR_NAME, path};
///
/// let state_dir = path::default_state_dir();
///
/// assert!(state_dir.ends_with(STATE_DIR_NAME));
/// ```
pub fn default_state_dir() -> PathBuf {
    scoped_join(&dirs::cache_dir().unwrap_or_default(), STATE_DIR_NAME).unwrap()
}

/// Joins the state name `name` under `root`, one path component per `/`-separated part
///
/// The name is validated first, so the result stays lexically inside `root`. Symlinks and
/// aliases are checked by [`resolves_in_place`]
///
/// # Errors
///
/// Returns [`NameError::Empty`], [`NameError::Escapes`] or [`NameError::Invalid`] when `name`
/// is not a valid state name, as [`validate_name`] decides
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use preader::path;
///
/// let joined = path::scoped_join(Path::new("states"), "logs/job-1")?;
///
/// assert_eq!(joined, Path::new("states/logs/job-1"));
/// # Ok::<(), preader::Error>(())
/// ```
pub fn scoped_join(root: &Path, name: &str) -> Result<PathBuf, NameError> {
    Ok(root.join(validate_name(name)?.split(NAME_SEPARATOR).collect::<PathBuf>()))
}

/// Returns whether `path` under `root` resolves to itself rather than to an alias of another entry
///
/// Only the part of `path` below `root` is checked: its deepest existing ancestor must equal its
/// canonical form, so a symlink or a case or Unicode alias below `root` makes it `false`. It is
/// `true` when `root` cannot be canonicalized (for example because it does not exist yet) or
/// `path` is not under `root`
///
/// # Examples
///
/// ```
/// use preader::path;
///
/// let dir = tempfile::tempdir()?;
/// let file = dir.path().join("logs").join("job-1.state.json");
///
/// assert!(path::resolves_in_place(dir.path(), &file));
/// # Ok::<(), preader::Error>(())
/// ```
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
