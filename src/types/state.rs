use std::{
    fs,
    path::{Path, PathBuf},
};

use chrono::Utc;
use derive_more::{Deref, From, PartialEq};
use serde::{Deserialize, Serialize};
use serde_json::to_string_pretty;

use crate::{
    Mismatch, Result,
    manager::StateManager,
    types::{checksum::ChecksumBody, file::FileMetadata, time::Timestamps},
};

#[derive(Clone, Debug, Deref, From, PartialEq)]
pub struct State {
    #[deref]
    pub(crate) data: StateData,
    #[partial_eq(skip)]
    pub(crate) manager: StateManager,
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
    pub(crate) fn new(manager: StateManager, path: &Path, name: String) -> Result<Self> {
        let mut data = StateData {
            name,
            file: FileMetadata::try_from(path)?,
            position: 0,
            timestamps: Timestamps::now(),
            checksum: String::new(),
        };

        data.checksum = ChecksumBody::from(&data).compute()?;

        Ok(Self { data, manager })
    }

    pub fn path(&self) -> Result<PathBuf> {
        self.manager.path(&self.name)
    }

    pub fn checksum(&self) -> Result<String> {
        ChecksumBody::from(&self.data).compute()
    }

    pub fn percent(&self) -> f64 {
        self.position as f64 * 100.0 / self.file.size.max(1) as f64
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

        let current = FileMetadata::try_from(self.file.path.as_path())?;

        Ok(self.file.compare(&current)?)
    }

    pub fn save(&mut self) -> Result<PathBuf> {
        let refreshed = self.refresh(None)?;
        let path = self.commit(&refreshed)?;

        self.data = refreshed;

        Ok(path)
    }

    pub fn resync(&self, path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self::from((
            self.refresh(Some(&dunce::canonicalize(path)?))?,
            self.manager.clone(),
        )))
    }

    fn refresh(&self, file: Option<&Path>) -> Result<StateData> {
        let mut data = self.data.clone();

        if let Some(path) = file {
            data.file = FileMetadata::try_from(path)?;
        }

        data.timestamps.updated_at = Utc::now();
        data.checksum = ChecksumBody::from(&data).compute()?;

        Ok(data)
    }

    pub(crate) fn advance(&mut self, bytes: u64) {
        self.data.position += bytes;
    }

    pub(crate) fn seek(mut self, position: u64) -> Self {
        self.data.position = position;
        self
    }

    fn commit(&self, data: &StateData) -> Result<PathBuf> {
        let (path, tmp) = (self.path()?, self.manager.tmp(&self.name)?);

        path.parent().map(fs::create_dir_all).transpose()?;

        fs::write(&tmp, to_string_pretty(data)?)
            .and_then(|()| fs::rename(&tmp, &path))
            .inspect_err(|_| drop(fs::remove_file(&tmp)))?;

        Ok(path)
    }
}
