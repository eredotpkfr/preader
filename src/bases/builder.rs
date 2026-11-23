use std::path::PathBuf;

use crate::{
    Config, IteratorOptions, Result, StateRegistry, StateSource,
    bases::iterator::PReaderIterator,
    interfaces::{iterator::IteratorRead, skip::Skip},
};

#[derive(Debug)]
#[must_use]
pub struct PReaderIteratorBuilder<'a, I> {
    pub(crate) config: &'a Config,
    pub(crate) registry: &'a StateRegistry,
    pub(crate) options: IteratorOptions,
    pub(crate) state: StateSource,
    pub(crate) file: PathBuf,
    pub(crate) inner: I,
}

impl<'a, I: Default> PReaderIteratorBuilder<'a, I> {
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
    pub fn build(self) -> Result<PReaderIterator<I>> {
        self.options.validate()?;

        let file = dunce::canonicalize(&self.file)?;
        let skip = self.inner.skip(self.options.skip);

        let state = self.state.resolve(self.registry, &file)?;
        let window = self.options.window(state.position, state.file.size, skip);

        PReaderIterator::new(self.config, self.registry, state, &window, self.inner)
    }
}
