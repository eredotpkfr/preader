use std::{io::Seek, ops::Range};

use crate::{
    AutoSave, Config, IteratorOptions, Progress, Result, State,
    enums::skip::Skip,
    interfaces::{iterator::IteratorRead, segmented::Segmented},
    types::{core::FileReader, window::Window},
};

#[derive(Debug)]
pub struct PReaderIterator<I> {
    pub(crate) reader: FileReader,
    pub(crate) progress: Progress,
    pub(crate) state: State,
    pub(crate) autosave: AutoSave,
    pub(crate) saved: u64,
    pub(crate) inner: I,
}

impl<I> PReaderIterator<I> {
    pub(crate) fn new(
        config: &Config,
        options: IteratorOptions,
        state: State,
        window: &Window,
        skip: Skip,
        inner: I,
    ) -> Result<Self> {
        let reader = window.open(&state.file.path, config.buffer_capacity)?;
        let progress = window.progress(&state.file.path, options, skip)?;
        let autosave = AutoSave::from(config);
        let saved = autosave.floor(window.position);
        let state = state.seek(window.position);

        Ok(Self {
            reader,
            progress,
            state,
            autosave,
            saved,
            inner,
        })
    }

    pub fn state(&mut self) -> &mut State {
        &mut self.state
    }

    pub(crate) fn done(&self) -> bool {
        self.progress.done(self.state.position)
    }

    pub(crate) fn advance(&mut self, read: Result<usize>) -> Result<usize> {
        if read.is_err() {
            let consumed = self.reader.stream_position()?;

            self.state.advance(consumed.saturating_sub(self.state.position));

            return read;
        }

        let bytes = read?;

        self.state.advance(bytes as u64);

        match self.autosave {
            AutoSave::Every(threshold) => self.save(threshold)?,
            AutoSave::Off | AutoSave::Final => (),
        }

        Ok(bytes)
    }

    pub(crate) fn stop<T>(&mut self) -> Result<Option<T>> {
        self.save(0)?;
        Ok(None)
    }

    fn save(&mut self, threshold: u64) -> Result<()> {
        let pending = self.state.position.saturating_sub(self.saved);

        if !self.autosave.active() || pending < threshold.max(1) {
            return Ok(());
        }

        self.state.save().inspect_err(|_| {
            self.autosave = AutoSave::Off;
            self.progress.exhaust();
        })?;

        self.saved = self.state.position;

        Ok(())
    }
}

impl<I: Segmented> PReaderIterator<I> {
    pub(crate) fn segment(&mut self) -> Result<Option<Range<usize>>> {
        loop {
            if self.done() {
                return self.stop();
            }

            let read = self.inner.fill(&mut self.reader);

            if self.advance(read)? == 0 {
                return self.stop();
            }

            if self.progress.skip() {
                continue;
            }

            if let Some(body) = self.inner.body() {
                self.progress.count();

                return Ok(Some(body));
            }
        }
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
        if let Err(error) = self.save(0) {
            eprintln!("preader: save failed: {error}");
        }
    }
}
