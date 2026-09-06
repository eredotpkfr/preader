#[derive(Debug, thiserror::Error)]
pub enum PathError {
    #[error("path must not be empty")]
    Empty,
    #[error("path escapes root: {0}")]
    Escapes(String),
    #[error("path must name an entry: {0}")]
    Nameless(String),
    #[error("path escapes root via symlink: {0}")]
    Symlink(String),
}
