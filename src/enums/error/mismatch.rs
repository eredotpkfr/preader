// Hints name the remedy, not an API call: the same message reaches Rust and Python callers
const RESYNC_HINT: &str = "(resync the state if this is expected)";
const MOVED_HINT: &str = "(resync the state if the file moved)";
const RESTART_HINT: &str = "(start over under a new state)";

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Mismatch {
    #[error("state file was modified outside preader")]
    Checksum,
    #[error(
        "state object has unsaved progress or was modified outside preader \
         (pass the state returned by save())"
    )]
    Unsealed,
    #[error(
        "state file was saved elsewhere since this iterator last loaded or saved it \
         (open a new iterator to continue)"
    )]
    Stale,
    #[error("file path changed {MOVED_HINT}")]
    Path,
    #[error("file size changed {RESYNC_HINT}")]
    Size,
    #[error("file mtime changed {RESYNC_HINT}")]
    Mtime,
    #[error("file is missing {MOVED_HINT}")]
    Missing,
    #[error("file is not the tracked file {RESTART_HINT}")]
    Identity,
    #[error("state file belongs to '{saved}'")]
    Name { saved: String },
}
