use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use time::UtcDateTime;

use crate::{
    Error, Result,
    constants::FINGERPRINT_SAMPLE_BYTES,
    macros::ensure,
    utils::{self, file},
};

/// Path, size, modification time and fingerprint of a state's tracked file
///
/// The metadata is recorded when the state is created or resynced and is never refreshed by a
/// save. After the file grows, resuming reads only up to `size`, or fails with a size mismatch
/// when `verify_state` is on, until a resync records the file again
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "preader", eq, frozen, get_all)
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct FileMetadata {
    /// Path of the file, canonical when recorded by an iterator or a resync
    pub path: PathBuf,
    /// Size of the file in bytes, where reads stop
    pub size: u64,
    /// Modification time of the file, at microsecond precision
    #[serde(with = "crate::serializers::time::microseconds")]
    pub mtime: UtcDateTime,
    /// SHA-256 hex digest of the first 4096 bytes of the file
    pub fingerprint: String,
}

impl TryFrom<&Path> for FileMetadata {
    /// The error returned when the file cannot be read
    type Error = Error;

    /// Reads the metadata and fingerprint of the regular file at `path`
    ///
    /// `path` is stored as given, without canonicalization, and a symlink is followed to its
    /// target
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the metadata of `path` cannot be read or the file cannot be
    /// opened or read, with [`ErrorKind::IsADirectory`] when `path` is a directory and with
    /// [`ErrorKind::InvalidInput`] when it is not a regular file, and [`Error::InvalidMtime`] when
    /// the file's mtime is out of range
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::FileMetadata;
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// let metadata = FileMetadata::try_from(file.as_path())?;
    ///
    /// assert_eq!(metadata.size, 4);
    /// # Ok::<(), preader::Error>(())
    /// ```
    fn try_from(path: &Path) -> Result<Self> {
        let metadata = fs::metadata(path)?;

        ensure!(!metadata.is_dir(), io::Error::from(ErrorKind::IsADirectory));
        ensure!(
            metadata.is_file(),
            io::Error::new(ErrorKind::InvalidInput, "not a regular file")
        );

        Ok(Self {
            path: path.to_path_buf(),
            size: metadata.len(),
            mtime: utils::time::mtime(&metadata)?,
            fingerprint: file::fingerprint(path, metadata.len().min(FINGERPRINT_SAMPLE_BYTES))?,
        })
    }
}

impl TryFrom<&PathBuf> for FileMetadata {
    /// The error returned when the file cannot be read
    type Error = Error;

    /// Reads the metadata and fingerprint of the regular file at `path`
    ///
    /// Behaves like the conversion from `&Path`
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when `path` is a directory, is not a regular file, or cannot be
    /// read, and [`Error::InvalidMtime`] when the file's mtime is out of range
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::FileMetadata;
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// let metadata = FileMetadata::try_from(&file)?;
    ///
    /// assert_eq!(metadata.path, file);
    /// # Ok::<(), preader::Error>(())
    /// ```
    fn try_from(path: &PathBuf) -> Result<Self> {
        Self::try_from(path.as_path())
    }
}

impl FileMetadata {
    /// Returns whether the first bytes of `path` still hash to the recorded fingerprint
    ///
    /// Only the first [`size`](Self::size) bytes, at most [`FINGERPRINT_SAMPLE_BYTES`], are hashed,
    /// so a file that grew is still identified
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when `path` cannot be opened or read
    pub(crate) fn identifies(&self, path: &Path) -> Result<bool> {
        Ok(file::fingerprint(path, self.size.min(FINGERPRINT_SAMPLE_BYTES))? == self.fingerprint)
    }
}
