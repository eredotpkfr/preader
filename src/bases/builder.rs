use std::path::PathBuf;

use crate::{
    Config, IteratorOptions, Result, StateInput,
    bases::iterator::PReaderIterator,
    interfaces::{builder::IteratorBuild, iterator::IteratorRead, skippable::Skippable},
    manager::StateManager,
};

#[derive(Debug)]
pub struct PReaderIteratorBuilder<'a, I> {
    pub(crate) config: &'a Config,
    pub(crate) manager: &'a StateManager,
    pub(crate) options: IteratorOptions,
    pub(crate) state: StateInput,
    pub(crate) file: PathBuf,
    pub(crate) inner: I,
}

impl<'a, I: Default> PReaderIteratorBuilder<'a, I> {
    pub(crate) fn new(
        config: &'a Config,
        manager: &'a StateManager,
        file: impl Into<PathBuf>,
    ) -> Self {
        Self {
            config,
            manager,
            file: file.into(),
            state: StateInput::default(),
            options: IteratorOptions::default(),
            inner: I::default(),
        }
    }
}

impl<I: Skippable> IteratorBuild for PReaderIteratorBuilder<'_, I>
where
    PReaderIterator<I>: IteratorRead,
{
    type Iterator = PReaderIterator<I>;

    fn build(self) -> Result<Self::Iterator> {
        self.options.validate()?;

        let file = dunce::canonicalize(&self.file)?;
        let skip = self.inner.skip(self.options.skip);

        let state = self.state.resolve(self.manager, &file)?;
        let window = self.options.window(state.position, state.file.size, skip);

        PReaderIterator::new(self.config, self.options, state, &window, skip, self.inner)
    }
}
