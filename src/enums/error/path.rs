#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PathError {
    #[error("path must not be empty")]
    Empty,
    #[error("path escapes root: {0}")]
    Escapes(String),
    #[error("path is invalid: {0}")]
    Invalid(String),
    #[error("path is a symlink or an alias of another entry: {0}")]
    Alias(String),
}
