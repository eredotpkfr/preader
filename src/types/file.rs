use std::{
    fs,
    io::{self, ErrorKind},
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

        ensure!(!metadata.is_dir(), io::Error::from(ErrorKind::IsADirectory));
        ensure!(
            metadata.is_file(),
            io::Error::new(ErrorKind::InvalidInput, "not a regular file")
        );

        let seconds = match metadata.modified()?.duration_since(UNIX_EPOCH) {
            Ok(after) => after.as_secs().cast_signed(),
            Err(before) => 0_i64.saturating_sub_unsigned(
                before.duration().as_secs() + u64::from(before.duration().subsec_nanos() > 0),
            ),
        };
        let mtime = DateTime::<Utc>::from_timestamp(seconds, 0);

        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            mtime: mtime.ok_or(Error::InvalidMtime)?,
            fingerprint: fingerprint(path, FINGERPRINT_SAMPLE_BYTES)?,
        })
    }
}

impl FileMetadata {
    pub(crate) fn matches(&self, path: &Path) -> Result<bool> {
        Ok(fingerprint(path, self.size.min(FINGERPRINT_SAMPLE_BYTES))? == self.fingerprint)
    }

    pub(crate) fn compare(&self, current: &Self) -> Result<()> {
        ensure!(self.size == current.size, Mismatch::Size);
        ensure!(self.mtime == current.mtime, Mismatch::Mtime);
        ensure!(
            self.fingerprint == current.fingerprint,
            Mismatch::Fingerprint
        );

        Ok(())
    }
}
