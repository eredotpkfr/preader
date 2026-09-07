use std::path::PathBuf;

// Advice appended to every mismatch that resyncing the state can resolve
const RESYNC_HINT: &str = "(call state.resync(file) if this is expected)";

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
}
