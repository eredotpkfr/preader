use std::path::{
    Component::{CurDir, ParentDir, Prefix, RootDir},
    Path, PathBuf,
};

use crate::PathError;

pub const DEFAULT_STATE_DIRECTORY: &str = "preader";

pub fn default_state_dir() -> PathBuf {
    scoped_join(
        &dirs::cache_dir().unwrap_or_default(),
        DEFAULT_STATE_DIRECTORY,
    )
    .unwrap()
}

pub fn scoped_join(root: &Path, unsafe_path: &str) -> Result<PathBuf, PathError> {
    if unsafe_path.is_empty() {
        return Err(PathError::Empty);
    }

    let candidate = Path::new(unsafe_path);
    let escapes = candidate
        .components()
        .any(|component| matches!(component, ParentDir | Prefix(_) | RootDir));

    if escapes {
        return Err(PathError::Escapes(unsafe_path.to_owned()));
    }

    if candidate.file_name().is_none() {
        return Err(PathError::Nameless(unsafe_path.to_owned()));
    }

    Ok(root.join(unsafe_path))
}

pub fn has_no_symlinks(root: &Path, path: &Path) -> bool {
    let (Ok(root), Ok(relative)) = (root.canonicalize(), path.strip_prefix(root)) else {
        return true;
    };

    root.join(relative)
        .ancestors()
        .find(|ancestor| ancestor.exists())
        .is_none_or(|existing| existing.canonicalize().is_ok_and(|real| real == existing))
}

pub fn strip_extensions<'a>(name: &'a str, extension: &str) -> &'a str {
    name.trim_end_matches(&format!(".{extension}"))
}

pub fn normalize_path(path: &str) -> String {
    Path::new(path)
        .components()
        .filter(|component| component != &CurDir)
        .collect::<PathBuf>()
        .to_string_lossy()
        .into_owned()
}

pub fn path_stem(path: &str, extension: &str) -> String {
    let normalized = normalize_path(path);
    let candidate = Path::new(&normalized);

    let Some(last) = candidate.file_name().and_then(|last| last.to_str()) else {
        return normalized;
    };
    let stem = strip_extensions(last, extension);

    if Path::new(stem).file_name().and_then(|stem| stem.to_str()) != Some(stem) {
        return String::new();
    }

    candidate.with_file_name(stem).to_string_lossy().into_owned()
}
