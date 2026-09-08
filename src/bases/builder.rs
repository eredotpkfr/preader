use std::path::PathBuf;

use crate::{
    AutoSave, Config, IteratorOptions, Result, StateInput, StateManager,
    bases::iterator::PReaderIterator,
    interfaces::{builder::IteratorBuild, iterator::IteratorRead, skippable::Skippable},
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
        let reader = window.open(&state.file.path, self.config.buffer_capacity)?;

        let progress = window.progress(reader.get_ref(), skip.boundary(), self.options.limit)?;
        let autosave = AutoSave::from(self.config);
        let state = state.seek(window.position);
        let saved = autosave.floor(window.position);

        Ok(PReaderIterator {
            reader,
            progress,
            autosave,
            saved,
            failure: None,
            state,
            inner: self.inner,
        })
    }
}
