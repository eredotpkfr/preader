#[derive(Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum NameError {
    #[error("state name must not be empty")]
    Empty,
    #[error("state name escapes the state directory")]
    Escapes,
    #[error("state name is not valid")]
    Invalid,
    #[error("state name is an alias of another entry (symlink, case or Unicode)")]
    Alias,
}
