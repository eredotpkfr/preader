use std::{
    fs,
    ops::{Deref, DerefMut},
    os::unix::fs::MetadataExt,
    path::PathBuf,
};

use anyhow::{Error, anyhow};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;

use crate::{
    Config, StateManager,
    exceptions::StateError,
    types::{checksum::ChecksumBody, file::FileMetadata, time::Timestamps},
    utils::fingerprint,
};

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct State {
    pub data: StateData,
    pub manager: StateManager,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct StateData {
    pub name: String,
    pub file: FileMetadata,
    pub position: u64,
    pub timestamps: Timestamps,
    #[serde(rename = "_checksum")]
    pub checksum: String,
}

impl Deref for State {
    type Target = StateData;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl DerefMut for State {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl From<(StateData, StateManager)> for State {
    fn from((data, manager): (StateData, StateManager)) -> Self {
        Self { data, manager }
    }
}

impl State {
    pub(crate) fn new(config: &Config, path: &PathBuf, name: String) -> Result<Self, Error> {
        let file = FileMetadata::try_from(path)?;
        let position = 0;
        let timestamps = Timestamps::now();
        let manager = StateManager::from(config);
        let checksum = ChecksumBody {
            name: &name,
            file: &file,
            position,
            timestamps: &timestamps,
        }
        .compute()?;
        let data = StateData {
            name,
            file,
            position,
            timestamps,
            checksum,
        };

        Ok(Self { data, manager })
    }

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
}

#[pymethods]
impl State {
    #[getter]
    fn name(&self) -> String {
        self.name.clone()
    }

    #[getter]
    fn file(&self) -> FileMetadata {
        self.file.clone()
    }

    #[getter]
    fn position(&self) -> u64 {
        self.position
    }

    #[getter]
    fn timestamps(&self) -> Timestamps {
        self.timestamps.clone()
    }

    fn checksum(&self) -> String {
        self.body().compute().unwrap()
    }

    pub fn percent(&self) -> f64 {
        self.position as f64 * 100.0 / self.file.size.max(1) as f64
    }

    pub fn save(&mut self) -> PyResult<PathBuf> {
        self.refresh();

        let path = self.manager.path(&self.name);
        let tmp = self.manager.tmp(&self.name);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let serialized = to_string_pretty(&self.data).map_err(StateError::from_serde)?;

        fs::write(&tmp, &serialized)?;
        fs::rename(&tmp, &path)?;

        self.manager.last_saved_position = self.position;

        Ok(path)
    }

    pub fn verify(&self) -> PyResult<()> {
        let computed = self.checksum();
        let metadata = fs::metadata(&self.file.path)?;
        let current_mtime = metadata.mtime();
        let current_fingerprint = fingerprint(&self.file.path)?;

        if computed != self.checksum {
            return Err(StateError::from_anyhow(anyhow!(
                "state checksum mismatch (saved: {}, computed: {})",
                self.checksum,
                computed
            )));
        }

        if self.file.size != metadata.len() {
            return Err(StateError::from_anyhow(anyhow!(
                "file size mismatch (saved: {}, current: {})",
                self.file.size,
                metadata.len(),
            )));
        }

        if self.file.mtime != current_mtime {
            return Err(StateError::from_anyhow(anyhow!(
                "file mtime mismatch (saved: {}, current: {})",
                self.file.mtime,
                current_mtime,
            )));
        }

        if self.file.fingerprint != current_fingerprint {
            return Err(StateError::from_anyhow(anyhow!(
                "file fingerprint mismatch (saved: {}, current: {})",
                self.file.fingerprint,
                current_fingerprint,
            )));
        }

        Ok(())
    }
}
