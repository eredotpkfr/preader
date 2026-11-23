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

#[derive(Debug)]
pub struct PReaderIterator<I> {
    pub(crate) reader: FileReader,
    pub(crate) progress: Progress,
    pub(crate) state: State,
    pub(crate) registry: StateRegistry,
    pub(crate) autosave: AutoSave,
    pub(crate) checkpoint: u64,
    pub(crate) synced: bool,
    pub(crate) inner: I,
}

impl<I> PReaderIterator<I> {
    pub(crate) fn new(
        config: &Config,
        registry: &StateRegistry,
        state: State,
        window: &Window,
        inner: I,
    ) -> Result<Self> {
        let reader = window.open(&state.file.path, config.buffer_capacity)?;
        let progress = window.progress(&state.file.path)?;
        let autosave = AutoSave::from(config);
        let checkpoint = autosave.checkpoint(window.position);
        let synced = registry.read(&state.name).is_ok_and(|saved| saved.checksum == state.checksum);
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

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn percent(&self) -> f64 {
        self.state.percent()
    }

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

    #[cfg(feature = "testing")]
    pub fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    pub(crate) fn done(&self) -> bool {
        self.progress.done(self.state.position)
    }

    pub(crate) fn advance(&mut self, read: Result<usize>) -> Result<usize> {
        self.state.advance(self.progress.take());

        if read.is_err() {
            let consumed = self.reader.stream_position()?.saturating_sub(self.state.position);

            self.progress.hold(consumed);

            return read;
        }

        let bytes = read?;

        self.progress.hold(bytes as u64);

        match self.autosave {
            AutoSave::EveryBytes(threshold) => self.autosave(threshold)?,
            AutoSave::Never => (),
        }

        Ok(bytes)
    }

    pub(crate) fn finalize<T>(&mut self) -> Result<Option<T>> {
        self.state.advance(self.progress.take());
        self.autosave(NonZeroU64::MIN)?;

        Ok(None)
    }

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
            self.progress.exhaust();

            return Err(error);
        }

        self.checkpoint = self.state.position;

        Ok(())
    }
}

impl<I: Segmented> PReaderIterator<I> {
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

    fn terminated(&mut self, boundary: u8) -> Result<bool> {
        self.reader.seek_relative(-1)?;

        let last = (&mut self.reader).bytes().next().transpose()?;

        Ok(last == Some(boundary))
    }
}

impl<I> Iterator for PReaderIterator<I>
where
    Self: IteratorRead,
{
    type Item = Result<<Self as IteratorRead>::Owned>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read().map(|item| item.map(Into::into)).transpose()
    }
}

impl<I> Drop for PReaderIterator<I> {
    fn drop(&mut self) {
        if let Err(error) = self.autosave(NonZeroU64::MIN) {
            eprintln!("preader: save failed: {error}");
        }
    }
}
