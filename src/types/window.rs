use std::{
    fs::{self, File},
    io::{self, BufReader, ErrorKind, Seek, SeekFrom},
    path::Path,
};

use crate::{Progress, Result, macros::ensure, types::core::FileReader, utils::file};

/// Resolved plan of one read: its start, its end, its item limit and the items to skip
///
/// Computed by `IteratorOptions::window` from the options, the skip plan, the resumed position
/// and the recorded file size
#[derive(Debug)]
pub struct Window {
    /// Byte offset the read starts at
    pub position: u64,
    /// Byte offset at which no new item may start, clamped to the recorded file size
    pub end: u64,
    /// Maximum number of items the read may yield
    pub limit: u64,
    /// Items to skip, or `None` when the resumed position is past the start or at or past the end
    pub skipping: Option<u64>,
    /// Boundary byte used to skip a partial first item, or `None` without alignment
    pub boundary: Option<u8>,
}

impl Window {
    /// Opens `path` positioned at the window start with a buffer of `capacity` bytes
    ///
    /// A `capacity` of `0` is raised to `1`. A FIFO, socket or device is rejected before it is
    /// opened, since opening one could block forever
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) when `path` is not a regular file or a directory, or
    /// when the file cannot be opened or moved to [`position`](Self::position)
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::Window;
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    /// let window = Window {
    ///     position: 3,
    ///     end: 6,
    ///     limit: u64::MAX,
    ///     skipping: None,
    ///     boundary: None,
    /// };
    ///
    /// std::fs::write(&file, "foobar")?;
    ///
    /// let text = std::io::read_to_string(window.open(&file, 8)?)?;
    ///
    /// assert_eq!(text, "bar");
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn open(&self, path: &Path, capacity: usize) -> Result<FileReader> {
        let file_type = fs::metadata(path)?.file_type();

        ensure!(
            file_type.is_file() || file_type.is_dir(),
            io::Error::new(ErrorKind::InvalidInput, "not a regular file")
        );

        let mut file = File::open(path)?;

        file.seek(SeekFrom::Start(self.position))?;

        Ok(BufReader::with_capacity(capacity.max(1), file))
    }

    /// Creates the [`Progress`] of a read over this window
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) when the byte before the start cannot be read to
    /// check alignment
    pub(crate) fn progress(&self, file: &File) -> Result<Progress> {
        Ok(Progress {
            end: self.end,
            limit: self.limit,
            yielded: 0,
            skipping: self.skip_items(file)?,
            pending: 0,
        })
    }

    /// Returns the number of items to skip, plus one when an aligned read starts mid-item
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) when the byte before the start cannot be read
    fn skip_items(&self, file: &File) -> Result<u64> {
        let Some(count) = self.skipping else {
            return Ok(0);
        };
        let Some(byte) = self.boundary else {
            return Ok(count);
        };

        Ok(count.saturating_add(u64::from(file::starts_mid_item(file, self.position, byte)?)))
    }
}
