use std::path::{
    Component::{ParentDir, Prefix},
    Path, PathBuf,
};

use anyhow::anyhow;

pub const DEFAULT_STATE_DIRECTORY: &str = "preader";

pub fn default_state_dir() -> PathBuf {
    scoped_join(
        &dirs::cache_dir().unwrap_or_default(),
        DEFAULT_STATE_DIRECTORY,
    )
    .unwrap()
}

pub fn scoped_join(root: &Path, unsafe_path: &str) -> anyhow::Result<PathBuf> {
    if unsafe_path.is_empty() {
        return Err(anyhow!("path must not be empty"));
    }

    let candidate = Path::new(unsafe_path);
    let components = candidate
        .components()
        .any(|component| matches!(component, ParentDir | Prefix(_)));

    if candidate.is_absolute() || components {
        return Err(anyhow!("path escapes root: {unsafe_path}"));
    }

    Ok(root.join(unsafe_path))
}
