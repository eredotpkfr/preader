use std::{fs::Metadata, time::UNIX_EPOCH};

use time::{SignedDuration, UtcDateTime};

use crate::{Error, Result};

pub fn now() -> UtcDateTime {
    UtcDateTime::now().truncate_to_second()
}

pub fn mtime(metadata: &Metadata) -> Result<UtcDateTime> {
    let delta = |duration| SignedDuration::try_from(duration).ok();

    match metadata.modified()?.duration_since(UNIX_EPOCH) {
        Ok(after) => delta(after).and_then(|after| UtcDateTime::UNIX_EPOCH.checked_add(after)),
        Err(before) => {
            delta(before.duration()).and_then(|before| UtcDateTime::UNIX_EPOCH.checked_sub(before))
        }
    }
    .map(UtcDateTime::truncate_to_microsecond) // matches the `microseconds` serializer, so a saved state reloads equal
    .ok_or(Error::InvalidMtime)
}
