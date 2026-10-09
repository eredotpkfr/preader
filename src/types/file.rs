use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    Error, Mismatch, Result, constants::FINGERPRINT_SAMPLE_BYTES, macros::ensure,
    utils::file::fingerprint,
};

#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "preader", eq, frozen, get_all, skip_from_py_object)
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub mtime: DateTime<Utc>,
    pub fingerprint: String,
}

impl TryFrom<&Path> for FileMetadata {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path)?;

        ensure!(metadata.is_file(), Error::NotAFile(path.to_path_buf()));

        let seconds = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs().cast_signed();
        let mtime = DateTime::<Utc>::from_timestamp(seconds, 0);

        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            mtime: mtime.ok_or(Error::InvalidMtime(seconds))?,
            fingerprint: fingerprint(path, FINGERPRINT_SAMPLE_BYTES)?,
        })
    }
}

impl FileMetadata {
    pub(crate) fn matches(&self, path: &Path) -> Result<bool> {
        Ok(fingerprint(path, self.size.min(FINGERPRINT_SAMPLE_BYTES))? == self.fingerprint)
    }

    pub(crate) fn compare(&self, current: &Self) -> Result<()> {
        ensure!(
            self.size == current.size,
            Mismatch::Size {
                saved: self.size,
                current: current.size
            }
        );
        ensure!(
            self.mtime == current.mtime,
            Mismatch::Mtime {
                saved: self.mtime.timestamp(),
                current: current.mtime.timestamp()
            }
        );
        ensure!(
            self.fingerprint == current.fingerprint,
            Mismatch::Fingerprint {
                saved: self.fingerprint.clone(),
                current: current.fingerprint.clone()
            }
        );

        Ok(())
    }
}
