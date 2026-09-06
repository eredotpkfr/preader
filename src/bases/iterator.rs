use std::ops::Range;

use crate::{
    Autosave, Progress, Result, State,
    interfaces::{iterator::IteratorRead, segmented::Segmented},
    types::core::Reader,
};

#[derive(Debug)]
pub struct PReaderIterator<F> {
    pub(crate) reader: Reader,
    pub(crate) progress: Progress,
    pub(crate) autosave: Autosave,
    pub(crate) failed: bool,
    pub(crate) state: State,
    pub(crate) fields: F,
}

impl<F> PReaderIterator<F> {
    pub fn state(&self) -> &State {
        &self.state
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
            Autosave::Every(threshold) => self.save(threshold),
            Autosave::Off | Autosave::Final => Ok(()),
        }
    }

    pub(crate) fn finish(&mut self) -> Result<()> {
        if self.failed || self.autosave == Autosave::Off {
            return Ok(());
        }

        self.save(0).inspect_err(|_| self.failed = true)
    }

    fn save(&mut self, threshold: u64) -> Result<()> {
        let pending = self.state.position - self.state.manager.last_saved_position;

        if pending > 0 && pending >= threshold {
            self.state.save()?;
        }

        Ok(())
    }
}

impl<F> Iterator for PReaderIterator<F>
where
    Self: IteratorRead,
{
    type Item = Result<<Self as IteratorRead>::Owned>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read().map(|item| item.map(Into::into)).transpose()
    }
}

impl<F: Segmented> PReaderIterator<F> {
    pub(crate) fn segment(&mut self) -> Result<Option<Range<usize>>> {
        loop {
            if self.done() {
                return self.stop();
            }

            let read = self.fields.fill(&mut self.reader)?;

            if read == 0 {
                return self.stop();
            }

            self.advance(read)?;

            if self.progress.skip() {
                continue;
            }

            if let Some(body) = self.fields.body() {
                self.progress.count();

                return Ok(Some(body));
            }
        }
    }
}

impl<F> Drop for PReaderIterator<F> {
    fn drop(&mut self) {
        if let Err(error) = self.finish() {
            eprintln!("preader: save failed: {error}");
        }
    }
}
