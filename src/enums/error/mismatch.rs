// Hints name the remedy, not an API call: the same message reaches Rust and Python callers
/// Remedy appended to messages about a file that changed in place
const RESYNC_HINT: &str = "(resync the state if this is expected)";
/// Remedy appended to messages about a file that moved or disappeared
const MOVED_HINT: &str = "(resync the state if the file moved)";
/// Remedy appended to messages about a file that was replaced by another one
const RESTART_HINT: &str = "(start over under a new state)";

/// Kind of disagreement reported by [`Error::Mismatch`]
///
/// [`Name`](Mismatch::Name) is checked whenever a state file is read, [`Path`](Mismatch::Path)
/// whenever a state starts an iterator, and [`Stale`](Mismatch::Stale) when an iterator that
/// holds the saved state saves again; the others need
/// [`Config::verify_state`](crate::Config::verify_state). With verification on, a state opened
/// on another file reports [`Identity`](Mismatch::Identity) instead of [`Path`](Mismatch::Path)
/// unless the new file starts like the tracked one. A file that grew, changed or moved is
/// accepted again with [`StateRegistry::resync`](crate::StateRegistry::resync); a file that is
/// no longer the tracked one needs a new state
///
/// # Examples
///
/// ```
/// use preader::{Config, Error, Mismatch, PReader};
///
/// let dir = tempfile::tempdir()?;
/// let file = dir.path().join("data.txt");
///
/// let reader = PReader::from(Config {
///     state_dir: dir.path().join("states"),
///     ..Config::default()
/// });
///
/// std::fs::write(&file, "foo\n")?;
///
/// reader.lines(&file).state("job-1").build()?.save()?;
///
/// std::fs::write(&file, "foo\nbar\n")?;
///
/// let kind = match reader.lines(&file).state("job-1").build() {
///     Err(Error::Mismatch { kind, .. }) => Some(kind),
///     _ => None,
/// };
///
/// assert!(matches!(kind, Some(Mismatch::Size)));
/// # Ok::<(), preader::Error>(())
/// ```
///
/// [`Error::Mismatch`]: crate::Error::Mismatch
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Mismatch {
    /// The state file's checksum does not match its content, so it was edited outside preader
    #[error("state file was modified outside preader")]
    Checksum,
    /// A [`State`](crate::State) passed to the builder has unsaved progress or was edited
    #[error(
        "state object has unsaved progress or was modified outside preader \
         (pass the state returned by save())"
    )]
    Unsealed,
    /// The state file was saved elsewhere since this iterator last loaded or saved it
    #[error(
        "state file was saved elsewhere since this iterator last loaded or saved it \
         (open a new iterator to continue)"
    )]
    Stale,
    /// The state tracks a different file path than the one being opened
    #[error("file path changed {MOVED_HINT}")]
    Path,
    /// The file's size differs from the size recorded in the state
    #[error("file size changed {RESYNC_HINT}")]
    Size,
    /// The file's modification time differs from the one recorded in the state
    #[error("file mtime changed {RESYNC_HINT}")]
    Mtime,
    /// The tracked file no longer exists
    #[error("file is missing {MOVED_HINT}")]
    Missing,
    /// The first bytes of the file no longer match the recorded fingerprint
    #[error("file is not the tracked file {RESTART_HINT}")]
    Identity,
    /// The state file records a different name than the one it was loaded under
    #[error("state file belongs to '{saved}'")]
    Name {
        /// Name recorded in the state file
        saved: String,
    },
}
