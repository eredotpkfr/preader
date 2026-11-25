use std::{fs::Metadata, time::UNIX_EPOCH};

use time::{SignedDuration, UtcDateTime};

use crate::{Error, Result};

/// Returns the current UTC time truncated to whole seconds
///
/// The truncation matches the precision [`Timestamps`](crate::Timestamps) are saved with
///
/// # Examples
///
/// ```
/// use preader::time;
///
/// let now = time::now();
///
/// assert_eq!(now.nanosecond(), 0);
/// ```
pub fn now() -> UtcDateTime {
    UtcDateTime::now().truncate_to_second()
}

/// Returns the modification time in `metadata` as UTC, truncated to whole microseconds
///
/// Times before the Unix epoch are supported. The truncation matches the precision of
/// [`FileMetadata::mtime`](crate::FileMetadata::mtime), so a saved mtime compares equal after it
/// is loaded again
///
/// # Errors
///
/// Returns [`Error::Io`] when the platform does not report modification times and
/// [`Error::InvalidMtime`] when the time cannot be represented as a `UtcDateTime`
///
/// # Examples
///
/// ```
/// use preader::time;
///
/// let dir = tempfile::tempdir()?;
/// let path = dir.path().join("data.txt");
///
/// std::fs::write(&path, "foo\n")?;
///
/// let mtime = time::mtime(&std::fs::metadata(&path)?)?;
///
/// assert_eq!(mtime, mtime.truncate_to_microsecond());
/// # Ok::<(), preader::Error>(())
/// ```
pub fn mtime(metadata: &Metadata) -> Result<UtcDateTime> {
    let delta = |duration| SignedDuration::try_from(duration).ok();

    match metadata.modified()?.duration_since(UNIX_EPOCH) {
        Ok(after) => delta(after).and_then(|after| UtcDateTime::UNIX_EPOCH.checked_add(after)),
        Err(before) => {
            delta(before.duration()).and_then(|before| UtcDateTime::UNIX_EPOCH.checked_sub(before))
        }
    }
    .map(UtcDateTime::truncate_to_microsecond)
    .ok_or(Error::InvalidMtime)
}
