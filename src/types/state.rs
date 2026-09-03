use std::{
    fs, io,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use anyhow::anyhow;
use derive_more::{Deref, DerefMut};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;

use crate::{
    Config, Error, StateManager,
    types::{checksum::ChecksumBody, file::FileMetadata, time::Timestamps},
    utils::file::fingerprint,
};

pub(crate) const RESYNC_HINT: &str = "(call state.resync(file) if this is expected)";

#[pyclass(module = "preader", eq, from_py_object)]
#[derive(Clone, Deref, DerefMut)]
pub struct State {
    #[deref]
    #[deref_mut]
    pub(crate) data: StateData,
    pub manager: StateManager,
}

#[derive(Clone, Deserialize, PartialEq, Serialize)]
pub struct StateData {
    pub name: String,
    pub file: FileMetadata,
    pub position: u64,
    pub timestamps: Timestamps,
    #[serde(rename = "_checksum")]
    pub checksum: String,
}

impl From<(StateData, StateManager)> for State {
    fn from((data, manager): (StateData, StateManager)) -> Self {
        Self { data, manager }
    }
}

impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data
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

    pub(crate) fn refresh(mut self) -> Result<Self, Error> {
        self.timestamps.updated_at = chrono::Utc::now();
        self.checksum = self.checksum()?;

        Ok(self)
    }

    fn commit(&self, tmp: &Path, path: &Path, serialized: &str) -> Result<(), Error> {
        fs::write(tmp, serialized)
            .and_then(|()| fs::rename(tmp, path))
            .inspect_err(|_| drop(fs::remove_file(tmp)))?;

        Ok(())
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

    fn path(&self) -> Result<PathBuf, Error> {
        self.manager.path(&self.name)
    }

    fn checksum(&self) -> Result<String, Error> {
        ChecksumBody::from(self).compute()
    }

    pub fn percent(&self) -> f64 {
        self.position as f64 * 100.0 / self.file.size.max(1) as f64
    }

    pub fn save(&mut self) -> Result<PathBuf, Error> {
        let path = self.path()?;
        let tmp = self.manager.tmp(&self.name)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let refreshed = self.clone().refresh()?;
        let serialized = to_string_pretty(&refreshed.data)?;

        self.commit(&tmp, &path, &serialized)?;

        self.manager.last_saved_position = refreshed.position;
        self.data = refreshed.data;

        Ok(path)
    }

    pub fn verify(&self) -> Result<(), Error> {
        let computed = self.checksum()?;
        let metadata = fs::metadata(&self.file.path)?;
        let current_mtime = metadata
            .modified()
            .and_then(|time| time.duration_since(UNIX_EPOCH).map_err(io::Error::other))?
            .as_secs() as i64;
        let saved_mtime = self.file.mtime.timestamp();
        let current_fingerprint = fingerprint(&self.file.path)?;

        if computed != self.checksum {
            return Err(Error::Message(anyhow!(
                "state checksum mismatch (saved: {}, computed: {})",
                self.checksum,
                computed
            )));
        }

        if self.file.size != metadata.len() {
            return Err(Error::Message(anyhow!(
                "file size mismatch (saved: {}, current: {}) {RESYNC_HINT}",
                self.file.size,
                metadata.len(),
            )));
        }

        if saved_mtime != current_mtime {
            return Err(Error::Message(anyhow!(
                "file mtime mismatch (saved: {}, current: {}) {RESYNC_HINT}",
                saved_mtime,
                current_mtime,
            )));
        }

        if self.file.fingerprint != current_fingerprint {
            return Err(Error::Message(anyhow!(
                "file fingerprint mismatch (saved: {}, current: {}) {RESYNC_HINT}",
                self.file.fingerprint,
                current_fingerprint,
            )));
        }

        Ok(())
    }

    pub fn resync(&self, path: PathBuf) -> Result<Self, Error> {
        let path = dunce::canonicalize(path)?;
        let file = FileMetadata::try_from(path.as_path())?;
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
