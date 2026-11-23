use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use time::UtcDateTime;

use crate::{
    Error, Result,
    constants::FINGERPRINT_SAMPLE_BYTES,
    macros::ensure,
    utils::{self, file},
};

#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "preader", eq, frozen, get_all)
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    #[serde(with = "crate::serializers::time::microseconds")]
    pub mtime: UtcDateTime,
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

        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            mtime: utils::time::mtime(&metadata)?,
            fingerprint: file::fingerprint(path, FINGERPRINT_SAMPLE_BYTES)?,
        })
    }
}

impl TryFrom<&PathBuf> for FileMetadata {
    type Error = Error;

    fn try_from(path: &PathBuf) -> Result<Self> {
        Self::try_from(path.as_path())
    }
}

impl FileMetadata {
    pub(crate) fn identifies(&self, path: &Path) -> Result<bool> {
        Ok(file::fingerprint(path, self.size.min(FINGERPRINT_SAMPLE_BYTES))? == self.fingerprint)
    }
}
