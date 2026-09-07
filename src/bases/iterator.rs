use std::ops::Range;

use crate::{
    AutoSave, Progress, Result, State,
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
    pub(crate) failed: bool,
    pub(crate) inner: I,
}

impl<I> PReaderIterator<I> {
    pub fn state(&mut self) -> &mut State {
        &mut self.state
    }

    pub(crate) fn done(&self) -> bool {
        self.progress.done(self.state.position)
    }

    pub(crate) fn stop<T>(&mut self) -> Result<Option<T>> {
        self.finish()?;

        Ok(None)
    }

    pub(crate) fn advance(&mut self, bytes: usize) -> Result<()> {
        self.state.advance(bytes as u64);

        match self.autosave {
            AutoSave::Every(threshold) => self.save(threshold),
            AutoSave::Off | AutoSave::Final => Ok(()),
        }
    }

    pub(crate) fn finish(&mut self) -> Result<()> {
        if self.failed || self.autosave == AutoSave::Off {
            return Ok(());
        }

        self.save(0).inspect_err(|_| self.failed = true)
    }

    fn save(&mut self, threshold: u64) -> Result<()> {
        let pending = self.state.position - self.saved;

        if pending > 0 && pending >= threshold {
            self.state.save()?;
            self.saved = self.state.position;
        }

        Ok(())
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

            self.advance(read)?;

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
        if let Err(error) = self.finish() {
            eprintln!("preader: save failed: {error}");
        }
    }
}
