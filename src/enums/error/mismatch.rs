// Advice appended to every mismatch that resyncing the state can resolve
const RESYNC_HINT: &str = "(call state.resync(file) if this is expected)";
// Advice appended to a mismatch that resyncing cannot resolve
const RESTART_HINT: &str = "(read it under a new state to start over)";

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Mismatch {
    #[error("state file was modified outside preader")]
    Checksum,
    #[error("state tracks a different file {RESYNC_HINT}")]
    Path,
    #[error("file size changed {RESYNC_HINT}")]
    Size,
    #[error("file mtime changed {RESYNC_HINT}")]
    Mtime,
    #[error("file content changed {RESYNC_HINT}")]
    Fingerprint,
    #[error("file is not the tracked file {RESTART_HINT}")]
    Identity,
    #[error("state file belongs to '{saved}'")]
    Name { saved: String },
}
