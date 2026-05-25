use std::{fs, path::PathBuf};

use anyhow::Error;
use pyo3::{exceptions::PyIOError, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    exceptions::PReaderStateError,
    types::{checksum::ChecksumBody, file::FileMetadata, time::Timestamps},
    utils::fingerprint,
};

#[pyclass(from_py_object)]
#[derive(Clone, Serialize, Deserialize)]
pub struct PReaderState {
    #[pyo3(get)]
    pub file: FileMetadata,
    #[pyo3(get)]
    pub position: u64,
    #[pyo3(get)]
    pub timestamps: Timestamps,
    #[pyo3(get)]
    #[serde(rename = "_checksum")]
    pub checksum: String,
}

impl TryFrom<FileMetadata> for PReaderState {
    type Error = Error;

    fn try_from(file: FileMetadata) -> Result<Self, Self::Error> {
        let position = 0;
        let timestamps = Timestamps::now();
        let checksum = ChecksumBody {
            file: &file,
            position,
            timestamps: &timestamps,
        }
        .compute()?;

        Ok(Self {
            file,
            position,
            timestamps,
            checksum,
        })
    }
}

impl TryFrom<&PathBuf> for PReaderState {
    type Error = Error;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        Ok(Self::try_from(FileMetadata::try_from(path)?)?)
    }
}

impl PReaderState {
    pub(crate) fn advance(&mut self, bytes: u64) {
        self.position += bytes
    }

    pub(crate) fn refresh(&mut self) {
        self.timestamps.updated_at = chrono::Utc::now().timestamp();
        self.checksum = self.checksum();
    }

    pub(crate) fn body(&self) -> ChecksumBody<'_> {
        self.into()
    }

    pub(crate) fn checksum(&self) -> String {
        self.body().compute().unwrap()
    }
}

#[pymethods]
impl PReaderState {
    pub fn percent(&self) -> f64 {
        self.position as f64 * 100.0 / self.file.size.max(1) as f64
    }

    pub fn verify_file(&self, path: PathBuf) -> PyResult<()> {
        let metadata = fs::metadata(&path).map_err(|e| PyIOError::new_err(e.to_string()))?;

        if self.file.size != metadata.len() {
            return Err(PReaderStateError::from_anyhow(Error::msg(format!(
                "size mismatch: expected {}, got {}",
                self.file.size,
                metadata.len()
            ))));
        }

        use std::os::unix::fs::MetadataExt;
        let current_mtime = metadata.mtime();
        if self.file.mtime != current_mtime {
            return Err(PReaderStateError::from_anyhow(Error::msg(format!(
                "mtime mismatch: expected {}, got {}",
                self.file.mtime, current_mtime
            ))));
        }

        let current_fingerprint =
            fingerprint(&path).map_err(|e| PyIOError::new_err(e.to_string()))?;
        if self.file.fingerprint != current_fingerprint {
            return Err(PReaderStateError::from_anyhow(Error::msg(format!(
                "fingerprint mismatch: expected {}, got {}",
                self.file.fingerprint, current_fingerprint
            ))));
        }

        Ok(())
    }
}
