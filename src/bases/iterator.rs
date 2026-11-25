use std::{
    io::{BufRead, Read, Seek},
    num::NonZeroU64,
    ops::Range,
};

use crate::{
    Config, Error, Mismatch, Progress, Result, State, StateRegistry,
    enums::autosave::AutoSave,
    interfaces::{iterator::IteratorRead, segmented::Segmented},
    macros::ensure,
    types::{core::FileReader, window::Window},
};

/// Resumable iterator over a file that tracks its progress in a [`State`]
///
/// The item type depends on `I`: [`ByteIterator`](crate::ByteIterator),
/// [`ChunkIterator`](crate::ChunkIterator), [`LineIterator`](crate::LineIterator) and
/// [`SegmentIterator`](crate::SegmentIterator) are the four forms, built through the
/// [`PReader`](crate::PReader) methods. Every item is a [`Result`]: a failed read is yielded and
/// the iteration continues, while a failed autosave or an I/O error that read nothing is yielded
/// once and ends it. [`save`](PReaderIterator::save) writes the progress to the reader's
/// [`StateRegistry`]. With [`Config::auto_save_state`] on it is also saved periodically, when the
/// iteration ends and on drop
#[derive(Debug)]
pub struct PReaderIterator<I> {
    /// Buffered handle on the tracked file, positioned after the bytes read so far
    pub(crate) reader: FileReader,
    /// Read bookkeeping: end, limit, items yielded and still to skip, and the pending bytes
    pub(crate) progress: Progress,
    /// State whose [`position`](field@State::position) is the end of the last finished item
    pub(crate) state: State,
    /// Registry the state is saved to
    pub(crate) registry: StateRegistry,
    /// Autosave policy, switched to [`AutoSave::Never`] after a failed autosave
    pub(crate) autosave: AutoSave,
    /// Position of the last autosave, or the baseline the first autosave is measured from
    ///
    /// The baseline is the resumed position rounded down when it matches the saved state, and 0
    /// otherwise, so a passed or moved state counts as unsaved progress unless it is at position 0
    pub(crate) checkpoint: u64,
    /// Whether the saved state had the checksum of [`state`] when it was last read or written
    ///
    /// [`state`]: field@Self::state
    pub(crate) synced: bool,
    /// Item-splitting strategy and its buffer
    pub(crate) inner: I,
}

impl<I> PReaderIterator<I> {
    /// Opens the tracked file at the window position and builds the iterator over `window`
    ///
    /// The iterator counts as synced when `registry` holds a saved state with the same checksum,
    /// which arms the [`Mismatch::Stale`] check of [`save`](PReaderIterator::save)
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the file is not a regular file or a directory, cannot be opened
    /// or moved to the window position, or when the byte before it cannot be read for alignment
    pub(crate) fn new(
        config: &Config,
        registry: &StateRegistry,
        state: State,
        window: &Window,
        inner: I,
    ) -> Result<Self> {
        let reader = window.open(&state.file.path, config.buffer_capacity)?;
        let progress = window.progress(reader.get_ref())?;
        let autosave = AutoSave::from(config);
        let synced = registry.read(&state.name).is_ok_and(|saved| saved.checksum == state.checksum);
        let checkpoint = if synced {
            autosave.checkpoint(state.position)
        } else {
            0
        };
        let state = state.seek(window.position);
        let registry = registry.clone();

        Ok(Self {
            reader,
            progress,
            state,
            registry,
            autosave,
            checkpoint,
            synced,
            inner,
        })
    }

    /// Returns the current state of the iterator
    ///
    /// Its [`position`](field@State::position) is the end of the last finished item. An item is
    /// finished once the next one is requested, so right after an item is yielded the position
    /// still points at its start. Once the position moves the state is unsealed, so a clone passed
    /// back fails with [`Mismatch::Unsealed`] while [`Config::verify_state`] is on. Resume from
    /// the [`State`] that [`save`](PReaderIterator::save) returns instead
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
    /// assert_eq!(lines.state().position, 4);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn state(&self) -> &State {
        &self.state
    }

    /// Returns the share of the file finished so far, from `0.0` to `100.0`
    ///
    /// This is [`State::percent`] of the current state, so it counts finished items only and
    /// stays below `100.0` while the last item is unfinished. It reaches `100.0` only when the
    /// read reaches the end of a non-empty file; [`end`](crate::PReaderIteratorBuilder::end),
    /// [`limit`](crate::PReaderIteratorBuilder::limit),
    /// [`drop_partial`](crate::ChunkBuilder::drop_partial) or a last line or segment held back
    /// because the file grew can stop it earlier
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
    /// assert_eq!(lines.percent(), 50.0);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn percent(&self) -> f64 {
        self.state.percent()
    }

    /// Saves the current state to the reader's state directory and returns it
    ///
    /// The state holds finished items only, so unless the iteration has ended the last yielded
    /// item is read again on resume. It gets a fresh [`updated_at`](crate::Timestamps::updated_at)
    /// and checksum and is written atomically, whether or not [`Config::auto_save_state`] is on.
    /// A failed manual save does not stop the iteration. An iterator that did not start from the
    /// saved state, such as an auto-named one with [`Config::auto_load_state`] off, checks for
    /// other writers only after its first save. That first save overwrites whatever is saved
    /// under its name. A saved state that was deleted, cannot be read, does not parse or belongs to
    /// another name is written over. A state is meant to have one writer at a time, so concurrent
    /// writers of one state must be coordinated by the caller
    ///
    /// # Errors
    ///
    /// Returns [`Error::Mismatch`] with [`Mismatch::Stale`] when the iterator started from or
    /// last wrote the saved state and another writer has changed it since. Returns
    /// [`Error::Name`] when the state's name is not a valid state name and [`Error::Alias`] when
    /// a part of it is a symlink or a case or Unicode alias. Returns [`Error::Io`] when the
    /// state directory cannot be created or the state file cannot be written, and
    /// [`Error::Serde`] when the state cannot be serialized
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
    /// let mut lines = reader.lines(&file).state("job-1").build()?;
    ///
    /// lines.next().transpose()?;
    /// lines.next().transpose()?;
    /// lines.save()?;
    ///
    /// assert_eq!(reader.states().load("job-1")?.position, 4);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn save(&mut self) -> Result<&State> {
        let refreshed = self.state.clone().refreshed()?;
        let name = &self.state.name;
        let synced = self.synced;

        if synced && let Ok(saved) = self.registry.read(name) {
            ensure!(
                saved.checksum == self.state.checksum,
                Error::mismatch(name, Mismatch::Stale)
            );
        }

        self.registry.commit(&refreshed)?;
        self.state = refreshed;
        self.synced = true;

        Ok(&self.state)
    }

    /// Returns the current state for in-place changes in tests
    ///
    /// Changing its [`position`](field@State::position) does not move the reader, which goes on
    /// from where it was. The bytes left to read are counted from the new position, so moving it
    /// forward ends the read that much earlier
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
    /// lines.state_mut().position = 4;
    ///
    /// let first = lines.next().transpose()?;
    ///
    /// assert_eq!(first.as_deref(), Some("foo"));
    /// # Ok::<(), preader::Error>(())
    /// ```
    #[cfg(feature = "testing")]
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    /// Returns whether the window end or the item limit has been reached
    pub(crate) fn done(&self) -> bool {
        self.progress.done(self.state.position)
    }

    /// Finishes the previous item, holds the bytes of `read` as the current one and may autosave
    ///
    /// When `read` failed, the bytes the reader consumed are held instead and no autosave runs,
    /// so the next call goes on after them. A failed read that consumed nothing exhausts the
    /// iterator, since reading again could only repeat it
    ///
    /// # Errors
    ///
    /// Returns the error of `read` unchanged, [`Error::Io`] when the reader position cannot be
    /// queried after a failed read, and the errors of [`save`](PReaderIterator::save) when an
    /// autosave fails
    pub(crate) fn advance(&mut self, read: Result<usize>) -> Result<usize> {
        self.state.advance(self.progress.take());

        let Ok(bytes) = read else {
            let consumed = self.reader.stream_position()?.saturating_sub(self.state.position);

            self.progress.hold(consumed);

            if consumed == 0 {
                self.progress.exhaust();
            }

            return read;
        };

        self.progress.hold(bytes as u64);

        if let AutoSave::EveryBytes(threshold) = self.autosave {
            self.autosave(threshold)?;
        }

        Ok(bytes)
    }

    /// Finishes the last item, autosaves any unsaved progress and returns `Ok(None)`
    ///
    /// The iterator is exhausted, so every later read returns `None` even when bytes of a held
    /// back item are already consumed from the reader
    ///
    /// # Errors
    ///
    /// Returns the errors of [`save`](PReaderIterator::save) when the final autosave fails
    pub(crate) fn finalize<T>(&mut self) -> Result<Option<T>> {
        self.state.advance(self.progress.take());
        self.progress.exhaust();
        self.autosave(NonZeroU64::MIN)?;

        Ok(None)
    }

    /// Saves the state when autosave is on and at least `threshold` bytes are past [`checkpoint`]
    ///
    /// A state that is synced and unchanged since it was loaded or saved is not written again. A
    /// failed save switches autosave off and drops the unfinished item, so it is read again on
    /// resume, unless that item is being skipped. It also exhausts the iterator, so the error is
    /// yielded once and every later read returns `None`
    ///
    /// # Errors
    ///
    /// Returns [`Error::Serde`] when the state checksum cannot be computed, and the errors of
    /// [`save`](PReaderIterator::save) when the save fails
    ///
    /// [`checkpoint`]: Self::checkpoint
    fn autosave(&mut self, threshold: NonZeroU64) -> Result<()> {
        let unsaved = self.state.position.saturating_sub(self.checkpoint);

        if !self.autosave.active() || unsaved < threshold.get() {
            return Ok(());
        }

        if self.synced && self.state.is_sealed()? {
            return Ok(());
        }

        if let Err(error) = self.save() {
            self.autosave = AutoSave::Never;

            if self.progress.skipping == 0 {
                self.progress.take();
            }

            self.progress.exhaust();

            return Err(error);
        }

        self.checkpoint = self.state.position;

        Ok(())
    }
}

impl<I: Segmented> PReaderIterator<I> {
    /// Reads the next delimited item into the inner buffer and returns its body range
    ///
    /// Items still to be skipped and items [`Segmented::body`] rejects are read and dropped. It
    /// returns `None` once the read is done
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the file cannot be read, and the errors of
    /// [`save`](PReaderIterator::save) when an autosave fails
    pub(crate) fn segment(&mut self) -> Result<Option<Range<usize>>> {
        loop {
            if self.done() {
                return self.finalize();
            }

            let read = self.scan();

            if self.advance(read)? == 0 {
                return self.finalize();
            }

            if self.progress.skip() {
                continue;
            }

            if let Some(body) = self.inner.body() {
                self.progress.tick();

                return Ok(Some(body));
            }
        }
    }

    /// Reads up to the next boundary and returns the number of bytes read
    ///
    /// It never reads past the recorded file size. An item that reaches that size without its
    /// boundary while the file has grown is held back by returning `0`, because its end is not
    /// known yet
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the file cannot be read
    fn scan(&mut self) -> Result<usize> {
        let boundary = self.inner.boundary();
        let left = self.state.file.size.saturating_sub(self.state.position + self.progress.pending);
        let mut limited = (&mut self.reader).take(left);

        let read = if self.progress.skipping > 0 {
            limited.skip_until(boundary)?
        } else {
            let buffer = self.inner.buffer();

            buffer.clear();
            limited.read_until(boundary, buffer)?
        };

        let cut = read as u64 == left && !self.terminated(boundary)?;

        if cut && !self.reader.fill_buf()?.is_empty() {
            return Ok(0);
        }

        Ok(read)
    }

    /// Returns whether the last byte read is `boundary`
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the reader cannot step back one byte or read it again
    fn terminated(&mut self, boundary: u8) -> Result<bool> {
        self.reader.seek_relative(-1)?;

        let last = (&mut self.reader).bytes().next().transpose()?;

        Ok(last == Some(boundary))
    }
}

/// Yields owned items, one [`Result`] per item
///
/// Each item is the [`IteratorRead::read`] item converted into its owned form
impl<I> Iterator for PReaderIterator<I>
where
    Self: IteratorRead,
{
    type Item = Result<<Self as IteratorRead>::Owned>;

    /// Reads the next item and returns it as an owned value
    ///
    /// A read error, such as invalid UTF-8 in a line, is yielded as `Some(Err(_))` and the next
    /// call continues with the following item. Like any item, a failed one is finished only once
    /// the next item is requested, so a state saved right after the error resumes at it and
    /// yields the error again. An I/O error that read nothing, such as on a tracked file replaced
    /// by a directory, and a failed autosave are yielded the same way, but then every later call
    /// returns `None`
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
    /// std::fs::write(&file, b"\xff\nbar\n")?;
    ///
    /// let mut lines = reader.lines(&file).build()?;
    ///
    /// lines.next();
    ///
    /// let second = lines.next().transpose()?;
    ///
    /// assert_eq!(second.as_deref(), Some("bar"));
    /// # Ok::<(), preader::Error>(())
    /// ```
    fn next(&mut self) -> Option<Self::Item> {
        self.read().map(|item| item.map(Into::into)).transpose()
    }
}

/// Autosaves the finished items when the iterator is dropped
impl<I> Drop for PReaderIterator<I> {
    /// Saves the progress not saved yet, when [`Config::auto_save_state`] is on
    ///
    /// Unless the iteration has ended, the last yielded item is not finished, so it is not saved.
    /// A position moved forward by [`start`] or a byte or chunk [`skip`] counts as progress, and
    /// so does a passed [`State`] that differs from the saved one and is not at position 0. An
    /// unread iterator can therefore still save and overwrite the saved state. A failed save is
    /// only printed to stderr as `preader: save failed: {error}`. Nothing is saved after an
    /// autosave failed
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
    ///     auto_save_state: true,
    ///     ..Config::default()
    /// };
    /// let reader = PReader::from(config);
    ///
    /// std::fs::write(&file, "foo\nbar\n")?;
    ///
    /// let mut lines = reader.lines(&file).state("job-1").build()?;
    ///
    /// lines.next().transpose()?;
    /// lines.next().transpose()?;
    /// drop(lines);
    ///
    /// assert_eq!(reader.states().load("job-1")?.position, 4);
    /// # Ok::<(), preader::Error>(())
    /// ```
    ///
    /// [`start`]: crate::PReaderIteratorBuilder::start
    /// [`skip`]: crate::PReaderIteratorBuilder::skip
    fn drop(&mut self) {
        if let Err(error) = self.autosave(NonZeroU64::MIN) {
            eprintln!("preader: save failed: {error}");
        }
    }
}
