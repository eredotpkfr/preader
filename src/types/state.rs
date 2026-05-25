use std::{fs, path::PathBuf};

use pyo3::{exceptions::PyIOError, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    exceptions::PReaderStateError,
    types::{
        checksum::ChecksumBody, file::FileMetadata, identity::compute_first_4kib_hash,
        time::Timestamps,
    },
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

impl From<FileMetadata> for PReaderState {
    fn from(file: FileMetadata) -> Self {
        let position = 0;
        let timestamps = Timestamps::now();
        let checksum = ChecksumBody {
            file: &file,
            position,
            timestamps: &timestamps,
        }
        .compute();

        Self {
            file,
            position,
            timestamps,
            checksum,
        }
    }
}

impl TryFrom<&PathBuf> for PReaderState {
    type Error = PyErr;

    fn try_from(path: &PathBuf) -> Result<Self, Self::Error> {
        Ok(FileMetadata::try_from(path)?.into())
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

    pub(crate) fn checksum(&self) -> String {
        ChecksumBody::from(self).compute()
    }
}

#[pymethods]
impl PReaderState {
    #[getter]
    pub fn percent(&self) -> f64 {
        self.position as f64 * 100.0 / self.file.size.max(1) as f64
    }

    pub fn verify_file(&self, path: PathBuf) -> PyResult<()> {
        let metadata = fs::metadata(&path).map_err(|e| PyIOError::new_err(e.to_string()))?;

        if self.file.size != metadata.len() {
            return Err(PReaderStateError::size_mismatch(
                self.file.size,
                metadata.len(),
            ));
        }

        use std::os::unix::fs::MetadataExt;
        let current_mtime = metadata.mtime();
        if self.file.mtime != current_mtime {
            return Err(PReaderStateError::mtime_mismatch(
                self.file.mtime as u128,
                current_mtime as u128,
            ));
        }

        let current_hash =
            compute_first_4kib_hash(&path).map_err(|e| PyIOError::new_err(e.to_string()))?;
        if self.file.sha256_first_4kib != current_hash {
            return Err(PReaderStateError::hash_mismatch(
                &self.file.sha256_first_4kib,
                &current_hash,
            ));
        }

        Ok(())
    }
}
