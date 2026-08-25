use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::{Error, anyhow};
use chrono::{DateTime, Utc};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::utils::file::fingerprint;

#[pyclass(from_py_object)]
#[derive(Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    #[pyo3(get)]
    pub path: PathBuf,
    #[pyo3(get)]
    pub size: u64,
    #[pyo3(get)]
    #[serde(with = "chrono::serde::ts_seconds")]
    pub mtime: DateTime<Utc>,
    #[pyo3(get)]
    pub fingerprint: String,
}

impl TryFrom<&Path> for FileMetadata {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        let metadata = fs::metadata(path)?;

        if !metadata.is_file() {
            return Err(anyhow!("not a file: {}", path.display()));
        }

        let seconds = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let mtime = DateTime::<Utc>::from_timestamp(seconds, 0);

        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            mtime: mtime.ok_or_else(|| anyhow!("invalid mtime: {seconds}"))?,
            fingerprint: fingerprint(path)?,
        })
    }
}

#[pymethods]
impl FileMetadata {
    pub fn __repr__(&self) -> String {
        crate::macros::pyrepr!("FileMetadata" {
            path = format!("'{}'", self.path.display()),
            size = self.size,
            mtime = self.mtime.timestamp(),
            fingerprint = format!("'{}'", self.fingerprint),
        })
    }
}
