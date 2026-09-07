use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{Error, Mismatch, utils::file::fingerprint};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub mtime: DateTime<Utc>,
    pub fingerprint: String,
}

impl FileMetadata {
    pub(crate) fn compare(&self, current: &Self) -> Result<(), Mismatch> {
        if self.size != current.size {
            return Err(Mismatch::Size {
                saved: self.size,
                current: current.size,
            });
        }

        if self.mtime != current.mtime {
            return Err(Mismatch::Mtime {
                saved: self.mtime.timestamp(),
                current: current.mtime.timestamp(),
            });
        }

        if self.fingerprint != current.fingerprint {
            return Err(Mismatch::Fingerprint {
                saved: self.fingerprint.clone(),
                current: current.fingerprint.clone(),
            });
        }

        Ok(())
    }
}

impl TryFrom<&Path> for FileMetadata {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let metadata = fs::metadata(path)?;

        if !metadata.is_file() {
            return Err(Error::NotAFile(path.to_path_buf()));
        }

        let seconds = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let mtime = DateTime::<Utc>::from_timestamp(seconds, 0);

        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            mtime: mtime.ok_or(Error::InvalidMtime(seconds))?,
            fingerprint: fingerprint(path)?,
        })
    }
}
