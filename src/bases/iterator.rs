use std::ops::Range;

use crate::{
    AutoSave, Error, Progress, Result, State,
    interfaces::{iterator::IteratorRead, segmented::Segmented},
    types::core::Reader,
};

#[derive(Debug)]
pub struct PReaderIterator<I> {
    pub(crate) reader: Reader,
    pub(crate) progress: Progress,
    pub(crate) state: State,
    pub(crate) autosave: AutoSave,
    pub(crate) saved: u64,
    pub(crate) failure: Option<Error>,
    pub(crate) inner: I,
}

impl<I> PReaderIterator<I> {
    pub fn state(&mut self) -> &mut State {
        &mut self.state
    }

    pub fn error(&mut self) -> Option<Error> {
        self.failure.take()
    }

    pub(crate) fn done(&self) -> bool {
        self.progress.done(self.state.position)
    }

    pub(crate) fn advance(&mut self, bytes: usize) {
        self.state.advance(bytes as u64);

        if let AutoSave::Every(threshold) = self.autosave {
            self.save(threshold);
        }
    }

    pub(crate) fn stop<T>(&mut self) -> Result<Option<T>> {
        self.save(0);
        Ok(None)
    }

    fn save(&mut self, threshold: u64) {
        let pending = self.state.position - self.saved;

        if !self.autosave.active() || pending == 0 || pending < threshold {
            return;
        }

        self.saved = self.state.position;
        self.failure = self.state.save().err();
    }
}

impl<I: Segmented> PReaderIterator<I> {
    pub(crate) fn segment(&mut self) -> Result<Option<Range<usize>>> {
        loop {
            if self.done() {
                return self.stop();
            }

            let read = self.inner.fill(&mut self.reader)?;

            if read == 0 {
                return self.stop();
            }

            self.advance(read);

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
        self.save(0);

        if let Some(error) = &self.failure {
            eprintln!("preader: save failed: {error}");
        }
    }
}
