use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use derive_more::{Deref, From, PartialEq};
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;

use crate::{
    Config, Mismatch, Result, StateManager,
    enums::autosave::Autosave,
    types::{checksum::ChecksumBody, file::FileMetadata, time::Timestamps},
};

#[derive(Clone, Debug, Deref, From, PartialEq)]
pub struct State {
    #[deref]
    pub(crate) data: StateData,
    #[partial_eq(skip)]
    pub manager: StateManager,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StateData {
    pub name: String,
    pub file: FileMetadata,
    pub position: u64,
    pub timestamps: Timestamps,
    #[serde(rename = "_checksum")]
    pub checksum: String,
}

impl State {
    pub fn path(&self) -> Result<PathBuf> {
        self.manager.path(&self.name)
    }

    pub fn checksum(&self) -> Result<String> {
        ChecksumBody::from(&self.data).compute()
    }

    pub fn percent(&self) -> f64 {
        self.position as f64 * 100.0 / self.file.size.max(1) as f64
    }

    pub fn save(&mut self) -> Result<PathBuf> {
        let path = self.path()?;
        let tmp = self.manager.tmp(&self.name)?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut data = self.data.clone();

        data.timestamps.updated_at = Utc::now();
        data.checksum = ChecksumBody::from(&data).compute()?;

        self.commit(&tmp, &path, &to_string_pretty(&data)?)?;

        self.manager.last_saved_position = data.position;
        self.data = data;

        Ok(path)
    }

    pub fn verify(&self) -> Result<()> {
        let computed = self.checksum()?;

        if computed != self.checksum {
            return Err(Mismatch::Checksum {
                saved: self.checksum.clone(),
                computed,
            }
            .into());
        }

        Ok(self.file.compare(&FileMetadata::try_from(self.file.path.as_path())?)?)
    }

    pub fn resync(&self, path: impl AsRef<Path>) -> Result<Self> {
        let path = dunce::canonicalize(path)?;
        let mut resynced = self.clone();

        resynced.data.file = FileMetadata::try_from(path.as_path())?;
        resynced.data.timestamps.updated_at = Utc::now();
        resynced.data.checksum = ChecksumBody::from(&resynced.data).compute()?;

        Ok(resynced)
    }

    pub(crate) fn new(config: &Config, path: &Path, name: String) -> Result<Self> {
        let file = FileMetadata::try_from(path)?;
        let data = StateData {
            name,
            file,
            position: 0,
            timestamps: Timestamps::now(),
            checksum: String::new(),
        };
        let mut state = Self {
            data,
            manager: StateManager::from(config),
        };

        state.data.checksum = state.checksum()?;

        Ok(state)
    }

    pub(crate) fn advance(&mut self, bytes: u64) {
        self.data.position += bytes;
    }

    pub(crate) fn seek(&mut self, position: u64, autosave: Autosave) {
        self.data.position = position;
        self.manager.last_saved_position = match autosave {
            Autosave::Every(threshold) => position - position % threshold,
            Autosave::Off | Autosave::Final => position,
        };
    }

    fn commit(&self, tmp: &Path, path: &Path, serialized: &str) -> Result<()> {
        fs::write(tmp, serialized)
            .and_then(|()| fs::rename(tmp, path))
            .inspect_err(|_| drop(fs::remove_file(tmp)))?;

        Ok(())
    }
}
