use std::path::PathBuf;

use crate::{
    Config, IteratorOptions, Result, StateRegistry, StateSource,
    bases::iterator::PReaderIterator,
    interfaces::{iterator::IteratorRead, skip::Skip},
};

/// Configures an iterator over one file and opens it with [`build`](Self::build)
///
/// Returned by [`PReader::bytes`](crate::PReader::bytes),
/// [`PReader::chunks`](crate::PReader::chunks), [`PReader::lines`](crate::PReader::lines) and
/// [`PReader::segments`](crate::PReader::segments). Every setter consumes the builder and returns
/// it, and nothing touches the file system until `build` is called
#[derive(Debug)]
#[must_use]
pub struct PReaderIteratorBuilder<'a, I> {
    /// Configuration of the reader that created the builder
    pub(crate) config: &'a Config,
    /// Registry the state is loaded from and later saved to
    pub(crate) registry: &'a StateRegistry,
    /// Range, skip and limit of the read
    pub(crate) options: IteratorOptions,
    /// Where the iterator takes its state from
    pub(crate) state: StateSource,
    /// Path of the file as given, canonicalized by [`build`](Self::build)
    pub(crate) file: PathBuf,
    /// Settings of the item kind, such as the chunk size or the delimiter
    pub(crate) inner: I,
}

impl<'a, I: Default> PReaderIteratorBuilder<'a, I> {
    /// Creates a builder for `file` with the default state source, options and item settings
    pub(crate) fn new(
        config: &'a Config,
        registry: &'a StateRegistry,
        file: impl Into<PathBuf>,
    ) -> Self {
        Self {
            config,
            registry,
            file: file.into(),
            state: StateSource::default(),
            options: IteratorOptions::default(),
            inner: I::default(),
        }
    }
}

impl<I: Skip> PReaderIteratorBuilder<'_, I>
where
    PReaderIterator<I>: IteratorRead,
{
    /// Resolves the state, checks it against the file and opens the file at the resumed position
    ///
    /// Nothing is written. Which state is resumed is described on [`StateSource`]. [`start`] plus
    /// any byte or chunk [`skip`] moves it forward when that offset is beyond its position
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidRange`] when [`start`] is greater than [`end`], [`Error::Io`] when
    /// the file is missing or cannot be read, or a saved state cannot be read, and
    /// [`Error::InvalidMtime`] when the file's mtime is out of range. A file that is not a regular
    /// file gives [`Error::Io`], except a directory resumed with [`Config::verify_state`] off,
    /// whose first read fails instead. Returns [`Error::Name`] when a name given to
    /// [`state`](Self::state) is not a valid state name, [`Error::Alias`] when a part of the state
    /// name is a symlink or a case or Unicode alias, [`Error::Corrupt`] when the saved state file
    /// does not parse, and [`Error::Serde`] when the file path is not valid UTF-8. Returns
    /// [`Error::Mismatch`] with [`Mismatch::Name`] when the saved state belongs to another name and
    /// with [`Mismatch::Path`] when the saved or passed state tracks another file, even with
    /// [`Config::verify_state`] off. With it on, it also returns [`Mismatch::Checksum`] for an
    /// edited saved state, [`Mismatch::Unsealed`] for a passed [`State`] with unsaved progress or
    /// edits, and [`Mismatch::Identity`], [`Mismatch::Size`] or [`Mismatch::Mtime`] when the file
    /// changed since the state was created or last resynced. The checksum and fingerprint checks
    /// run before the path check, so another file whose fingerprint differs gives
    /// [`Mismatch::Identity`] instead of [`Mismatch::Path`]
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\nbar\n")?;
    ///
    /// let lines = reader.lines(&file).start(4).build()?;
    ///
    /// assert_eq!(lines.state().position, 4);
    /// # Ok::<(), preader::Error>(())
    /// ```
    ///
    /// [`Error::InvalidRange`]: crate::Error::InvalidRange
    /// [`Error::Io`]: crate::Error::Io
    /// [`Error::InvalidMtime`]: crate::Error::InvalidMtime
    /// [`Error::Name`]: crate::Error::Name
    /// [`Error::Alias`]: crate::Error::Alias
    /// [`Error::Corrupt`]: crate::Error::Corrupt
    /// [`Error::Serde`]: crate::Error::Serde
    /// [`Error::Mismatch`]: crate::Error::Mismatch
    /// [`Mismatch::Name`]: crate::Mismatch::Name
    /// [`Mismatch::Path`]: crate::Mismatch::Path
    /// [`Mismatch::Checksum`]: crate::Mismatch::Checksum
    /// [`Mismatch::Unsealed`]: crate::Mismatch::Unsealed
    /// [`Mismatch::Identity`]: crate::Mismatch::Identity
    /// [`Mismatch::Size`]: crate::Mismatch::Size
    /// [`Mismatch::Mtime`]: crate::Mismatch::Mtime
    /// [`State`]: crate::State
    /// [`start`]: Self::start
    /// [`end`]: Self::end
    /// [`skip`]: Self::skip
    pub fn build(self) -> Result<PReaderIterator<I>> {
        self.options.validate()?;

        let file = dunce::canonicalize(&self.file)?;
        let skip = self.inner.skip(self.options.skip);

        let state = self.state.resolve(self.registry, &file)?;
        let window = self.options.window(state.position, state.file.size, skip);

        PReaderIterator::new(self.config, self.registry, state, &window, self.inner)
    }
}
