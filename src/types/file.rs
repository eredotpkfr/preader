use std::{fs, os::unix::fs::MetadataExt, path::PathBuf};

use anyhow::Error;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};

use crate::utils::fingerprint;

#[pyclass(from_py_object)]
#[derive(Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    #[pyo3(get)]
    pub path: PathBuf,
    #[pyo3(get)]
    pub size: u64,
    #[pyo3(get)]
    pub mtime: i64,
    #[pyo3(get)]
    pub fingerprint: String,
}

impl TryFrom<&PathBuf> for FileMetadata {
    type Error = Error;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        let metadata = fs::metadata(path)?;

        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            mtime: metadata.mtime(),
            fingerprint: fingerprint(path)?,
        })
    }
}
