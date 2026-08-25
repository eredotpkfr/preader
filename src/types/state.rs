use std::{
    fs,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::{Error, anyhow};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;

use crate::{
    Config, StateManager,
    exceptions::StateError,
    types::{checksum::ChecksumBody, file::FileMetadata, time::Timestamps},
    utils::file::fingerprint,
};

pub(crate) const RESYNC_HINT: &str = "(call state.resync(file) if this is expected)";

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct State {
    pub(crate) data: StateData,
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
    pub(crate) fn new(config: &Config, path: &Path, name: String) -> Result<Self, Error> {
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

    pub(crate) fn refresh(mut self) -> PyResult<Self> {
        self.timestamps.updated_at = chrono::Utc::now();
        self.checksum = self.checksum()?;

        Ok(self)
    }

    fn commit(&self, tmp: &Path, path: &Path, serialized: &str) -> PyResult<()> {
        fs::write(tmp, serialized)
            .and_then(|()| fs::rename(tmp, path))
            .inspect_err(|_| drop(fs::remove_file(tmp)))
            .map_err(StateError::from_io)
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

    #[getter]
    fn path(&self) -> PyResult<PathBuf> {
        self.manager.path(&self.name).map_err(StateError::from_anyhow)
    }

    fn checksum(&self) -> PyResult<String> {
        ChecksumBody::from(self).compute().map_err(StateError::from_anyhow)
    }

    pub fn percent(&self) -> f64 {
        self.position as f64 * 100.0 / self.file.size.max(1) as f64
    }

    pub fn save(&mut self) -> PyResult<PathBuf> {
        let path = self.path()?;
        let tmp = self.manager.tmp(&self.name).map_err(StateError::from_anyhow)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(StateError::from_io)?;
        }

        let refreshed = self.clone().refresh()?;
        let serialized = to_string_pretty(&refreshed.data).map_err(StateError::from_serde)?;

        self.commit(&tmp, &path, &serialized)?;

        self.manager.last_saved_position = refreshed.position;
        self.data = refreshed.data;

        Ok(path)
    }

    pub fn verify(&self) -> PyResult<()> {
        let computed = self.checksum()?;
        let metadata = fs::metadata(&self.file.path).map_err(StateError::from_io)?;
        let current_mtime = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_secs() as i64;
        let saved_mtime = self.file.mtime.timestamp();
        let current_fingerprint = fingerprint(&self.file.path).map_err(StateError::from_io)?;

        if computed != self.checksum {
            return Err(StateError::from_anyhow(anyhow!(
                "state checksum mismatch (saved: {}, computed: {})",
                self.checksum,
                computed
            )));
        }

        if self.file.size != metadata.len() {
            return Err(StateError::from_anyhow(anyhow!(
                "file size mismatch (saved: {}, current: {}) {RESYNC_HINT}",
                self.file.size,
                metadata.len(),
            )));
        }

        if saved_mtime != current_mtime {
            return Err(StateError::from_anyhow(anyhow!(
                "file mtime mismatch (saved: {}, current: {}) {RESYNC_HINT}",
                saved_mtime,
                current_mtime,
            )));
        }

        if self.file.fingerprint != current_fingerprint {
            return Err(StateError::from_anyhow(anyhow!(
                "file fingerprint mismatch (saved: {}, current: {}) {RESYNC_HINT}",
                self.file.fingerprint,
                current_fingerprint,
            )));
        }

        Ok(())
    }

    pub fn resync(&self, path: PathBuf) -> PyResult<Self> {
        let path = path.canonicalize().map_err(StateError::from_io)?;
        let file = FileMetadata::try_from(path.as_path()).map_err(StateError::from_anyhow)?;
        let mut resynced = self.clone();

        resynced.file = file;
        resynced.refresh()
    }

    pub fn __repr__(&self) -> String {
        crate::macros::pyrepr!("State" {
            name = format!("'{}'", self.name),
            file = self.file.__repr__(),
            position = self.position,
            timestamps = self.timestamps.__repr__(),
        })
    }
}
