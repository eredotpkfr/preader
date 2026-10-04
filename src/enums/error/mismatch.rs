use std::path::PathBuf;

// Advice appended to every mismatch that resyncing the state can resolve
const RESYNC_HINT: &str = "(call state.resync(file) if this is expected)";
// Advice appended to a mismatch that resyncing cannot resolve
const RESTART_HINT: &str = "(read it under a new state to start over)";

#[derive(Debug, thiserror::Error)]
pub enum Mismatch {
    #[error("state checksum mismatch (saved: {saved}, computed: {computed})")]
    Checksum { saved: String, computed: String },
    #[error("file path mismatch (saved: '{}', current: '{}') {RESYNC_HINT}", saved.display(), current.display())]
    Path { saved: PathBuf, current: PathBuf },
    #[error("file size mismatch (saved: {saved}, current: {current}) {RESYNC_HINT}")]
    Size { saved: u64, current: u64 },
    #[error("file mtime mismatch (saved: {saved}, current: {current}) {RESYNC_HINT}")]
    Mtime { saved: i64, current: i64 },
    #[error("file fingerprint mismatch (saved: {saved}, current: {current}) {RESYNC_HINT}")]
    Fingerprint { saved: String, current: String },
    #[error(
        "file content differs from the tracked file (saved: '{}', current: '{}') {RESTART_HINT}",
        saved.display(),
        current.display()
    )]
    Identity { saved: PathBuf, current: PathBuf },
}
