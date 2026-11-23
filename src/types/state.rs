use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    Error, Mismatch, Result,
    macros::ensure,
    types::{file::FileMetadata, time::Timestamps},
    utils::{self, hash},
};

#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "preader", eq, frozen, from_py_object)
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct State {
    pub name: String,
    pub file: FileMetadata,
    pub position: u64,
    pub timestamps: Timestamps,
    #[serde(rename = "_checksum", skip_serializing_if = "String::is_empty")]
    pub checksum: String,
}

impl State {
    pub(crate) fn new(path: &Path, name: String) -> Result<Self> {
        Self {
            name,
            file: FileMetadata::try_from(path)?,
            position: 0,
            timestamps: Timestamps::now(),
            checksum: String::new(),
        }
        .sealed()
    }

    pub fn checksum(&self) -> Result<String> {
        let body = Self {
            checksum: String::new(),
            ..self.clone()
        };

        Ok(hash::sha256(serde_json::to_string(&body)?))
    }

    pub fn percent(&self) -> f64 {
        self.position as f64 * 100.0 / self.file.size.max(1) as f64
    }

    pub(crate) fn verify(&self) -> Result<()> {
        let current = self.resync_metadata(&self.file.path, true)?;

        ensure!(
            self.file.size == current.size,
            Error::mismatch(&self.name, Mismatch::Size)
        );
        ensure!(
            self.file.mtime == current.mtime,
            Error::mismatch(&self.name, Mismatch::Mtime)
        );

        Ok(())
    }

    pub(crate) fn refreshed(mut self) -> Result<Self> {
        self.timestamps.updated_at = utils::time::now();
        self.sealed()
    }

    pub(crate) fn resynced(self, path: &Path, verify: bool) -> Result<Self> {
        let file = self.resync_metadata(path, verify)?;

        Self {
            position: self.position.min(file.size),
            file,
            ..self
        }
        .refreshed()
    }

    pub(crate) fn advance(&mut self, bytes: u64) {
        self.position += bytes;
    }

    pub(crate) fn seek(mut self, position: u64) -> Self {
        self.position = position;
        self
    }

    pub(crate) fn tracking(self, file: &Path, verify: bool) -> Result<Self> {
        if self.file.path != file {
            self.resync_metadata(file, verify)?;

            return Err(Error::mismatch(&self.name, Mismatch::Path));
        }

        Ok(self)
    }

    pub(crate) fn is_sealed(&self) -> Result<bool> {
        Ok(self.checksum()? == self.checksum)
    }

    fn resync_metadata(&self, path: &Path, verify: bool) -> Result<FileMetadata> {
        ensure!(
            !verify || self.is_sealed()?,
            Error::mismatch(&self.name, Mismatch::Checksum)
        );
        ensure!(
            fs::exists(path)?,
            Error::mismatch(&self.name, Mismatch::Missing)
        );

        let file = FileMetadata::try_from(path)?;

        ensure!(
            !verify || self.file.identifies(path)?,
            Error::mismatch(&self.name, Mismatch::Identity)
        );

        Ok(file)
    }

    fn sealed(mut self) -> Result<Self> {
        self.checksum = self.checksum()?;

        Ok(self)
    }
}
