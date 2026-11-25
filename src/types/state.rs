use std::{fs, io::ErrorKind, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    Error, Mismatch, Result,
    macros::ensure,
    types::{file::FileMetadata, time::Timestamps},
    utils::{self, hash},
};

/// Saved progress of an iterator over one file
///
/// Saving an iterator or loading from the registry returns one, and passing it back as the
/// `state` of a new iterator resumes from it
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "preader", eq, frozen, from_py_object)
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct State {
    /// Name the state is saved under
    pub name: String,
    /// Metadata of the tracked file, recorded when the state was created or last resynced
    pub file: FileMetadata,
    /// Byte offset just past the last finished item, where a resumed iterator starts
    pub position: u64,
    /// Times the state was created and last saved or resynced
    pub timestamps: Timestamps,
    /// SHA-256 hex digest of the other fields as last sealed, saved under the JSON key `_checksum`
    #[serde(rename = "_checksum", skip_serializing_if = "String::is_empty")]
    pub checksum: String,
}

impl State {
    /// Creates a sealed state at position 0 that tracks the file at `path`
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when `path` cannot be read, [`Error::InvalidMtime`] when its mtime is
    /// out of range, and [`Error::Serde`] when `path` is not valid UTF-8
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

    /// Computes the checksum of the state's current fields
    ///
    /// A state is sealed when the result equals its [`checksum`](field@State::checksum) field. A
    /// state changed since it was last sealed fails with [`Mismatch::Unsealed`] when it is passed
    /// to a builder with [`Config::verify_state`](crate::Config::verify_state) on
    ///
    /// # Errors
    ///
    /// Returns [`Error::Serde`] when the tracked file path is not valid UTF-8
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let config = Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// };
    /// let reader = PReader::from(config);
    ///
    /// std::fs::write(&file, "foo\nbar\n")?;
    ///
    /// let mut lines = reader.lines(&file).build()?;
    ///
    /// lines.next().transpose()?;
    /// lines.next().transpose()?;
    ///
    /// let state = lines.state();
    ///
    /// assert_ne!(state.checksum()?, state.checksum);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn checksum(&self) -> Result<String> {
        let body = Self {
            checksum: String::new(),
            ..self.clone()
        };

        Ok(hash::sha256(serde_json::to_string(&body)?))
    }

    /// Returns the position as a percentage of the tracked file's recorded size
    ///
    /// The position counts finished items only, and an empty file stays at `0.0`
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let config = Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// };
    /// let reader = PReader::from(config);
    ///
    /// std::fs::write(&file, "foo\nbar\n")?;
    ///
    /// let mut lines = reader.lines(&file).build()?;
    ///
    /// lines.next().transpose()?;
    /// lines.next().transpose()?;
    ///
    /// assert_eq!(lines.state().percent(), 50.0);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn percent(&self) -> f64 {
        self.position as f64 * 100.0 / self.file.size.max(1) as f64
    }

    /// Checks the state against the file it tracks, as done with [`Config::verify_state`] on
    ///
    /// # Errors
    ///
    /// Returns the errors of [`resync_metadata`](Self::resync_metadata) with verification on,
    /// then [`Error::Mismatch`] with [`Mismatch::Size`] or [`Mismatch::Mtime`] when the file's
    /// size or mtime changed
    ///
    /// [`Config::verify_state`]: crate::Config::verify_state
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

    /// Returns the state with [`updated_at`](Timestamps::updated_at) set to now and resealed
    ///
    /// # Errors
    ///
    /// Returns [`Error::Serde`] when the checksum cannot be computed
    pub(crate) fn refreshed(mut self) -> Result<Self> {
        self.timestamps.updated_at = utils::time::now();
        self.sealed()
    }

    /// Returns the state re-pointed at `path`, with the position clamped to the new size
    ///
    /// # Errors
    ///
    /// Returns the errors of [`resync_metadata`](Self::resync_metadata) and of
    /// [`refreshed`](Self::refreshed)
    pub(crate) fn resynced(self, path: &Path, verify: bool) -> Result<Self> {
        let file = self.resync_metadata(path, verify)?;

        Self {
            position: self.position.min(file.size),
            file,
            ..self
        }
        .refreshed()
    }

    /// Moves the position forward by `bytes` without resealing the state
    pub(crate) fn advance(&mut self, bytes: u64) {
        self.position += bytes;
    }

    /// Returns the state with its position set to `position`, without resealing it
    pub(crate) fn seek(mut self, position: u64) -> Self {
        self.position = position;
        self
    }

    /// Returns the state when it tracks `file`, a check made even with `verify` off
    ///
    /// # Errors
    ///
    /// When the state tracks another path, returns the errors of
    /// [`resync_metadata`](Self::resync_metadata) for `file` first, so a state that a resync would
    /// reject reports that reason, and otherwise [`Error::Mismatch`] with [`Mismatch::Path`]
    pub(crate) fn tracking(self, file: &Path, verify: bool) -> Result<Self> {
        if self.file.path != file {
            self.resync_metadata(file, verify)?;

            return Err(Error::mismatch(&self.name, Mismatch::Path));
        }

        Ok(self)
    }

    /// Returns whether the stored checksum matches the state's current fields
    ///
    /// # Errors
    ///
    /// Returns [`Error::Serde`] when the checksum cannot be computed
    pub(crate) fn is_sealed(&self) -> Result<bool> {
        Ok(self.checksum()? == self.checksum)
    }

    /// Reads fresh metadata for `path` after checking that the state may be resynced to it
    ///
    /// # Errors
    ///
    /// Returns [`Error::Mismatch`] with [`Mismatch::Checksum`] when `verify` is on and the state
    /// is not sealed, with [`Mismatch::Missing`] when `path` does not exist, and with
    /// [`Mismatch::Identity`] when `verify` is on and `path` does not match the fingerprint.
    /// Returns [`Error::Io`] when `path` cannot be read, [`Error::InvalidMtime`] when its mtime is
    /// out of range, and [`Error::Serde`] when `verify` is on and the recorded path is not valid
    /// UTF-8
    fn resync_metadata(&self, path: &Path, verify: bool) -> Result<FileMetadata> {
        ensure!(
            !verify || self.is_sealed()?,
            Error::mismatch(&self.name, Mismatch::Checksum)
        );

        let exists = match fs::exists(path) {
            Err(error) if error.kind() == ErrorKind::NotADirectory => false,
            exists => exists?,
        };

        ensure!(exists, Error::mismatch(&self.name, Mismatch::Missing));

        let file = FileMetadata::try_from(path)?;

        ensure!(
            !verify || self.file.identifies(path)?,
            Error::mismatch(&self.name, Mismatch::Identity)
        );

        Ok(file)
    }

    /// Returns the state with its checksum set to [`checksum`](Self::checksum)
    ///
    /// # Errors
    ///
    /// Returns [`Error::Serde`] when the checksum cannot be computed
    fn sealed(mut self) -> Result<Self> {
        self.checksum = self.checksum()?;

        Ok(self)
    }
}
