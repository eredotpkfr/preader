use std::path::PathBuf;

use crate::{
    Autosave, Config, IteratorOptions, Result, Skip, StateInput,
    bases::iterator::PReaderIterator,
    interfaces::{builder::IteratorBuild, iterator::IteratorRead},
};

#[derive(Debug)]
pub struct PReaderIteratorBuilder<'a, F> {
    pub(crate) config: &'a Config,
    pub(crate) options: IteratorOptions,
    pub(crate) state: StateInput,
    pub(crate) file: PathBuf,
    pub(crate) fields: F,
}

impl<'a, F: Default> PReaderIteratorBuilder<'a, F> {
    pub(crate) fn new(config: &'a Config, file: impl Into<PathBuf>) -> Self {
        Self {
            config,
            file: file.into(),
            state: StateInput::default(),
            options: IteratorOptions::default(),
            fields: F::default(),
        }
    }
}

impl<F> PReaderIteratorBuilder<'_, F> {
    pub fn state(mut self, state: impl Into<StateInput>) -> Self {
        self.state = state.into();
        self
    }

    pub fn options(mut self, options: IteratorOptions) -> Self {
        self.options = options;
        self
    }

    pub fn start(mut self, start: u64) -> Self {
        self.options.start = start;
        self
    }

    pub fn end(mut self, end: u64) -> Self {
        self.options.end = end;
        self
    }

    pub fn skip(mut self, skip: u64) -> Self {
        self.options.skip = skip;
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.options.limit = limit;
        self
    }
}

impl<F> IteratorBuild for PReaderIteratorBuilder<'_, F>
where
    for<'a> Skip: From<&'a PReaderIteratorBuilder<'a, F>>,
    PReaderIterator<F>: IteratorRead,
{
    type Iterator = PReaderIterator<F>;

    fn build(self) -> Result<Self::Iterator> {
        self.options.validate()?;

        let file = dunce::canonicalize(&self.file)?;
        let skip = Skip::from(&self);

        let mut state = self.state.resolve(self.config, &file)?;

        let window = self.options.window(state.position, state.file.size, skip);
        let reader = window.open(&state.file.path, self.config.buffer_capacity)?;

        let progress = window.progress(reader.get_ref(), skip.boundary(), self.options.limit)?;
        let autosave = Autosave::from(self.config);

        state.seek(window.position, autosave);

        Ok(PReaderIterator {
            state,
            reader,
            progress,
            autosave,
            failed: false,
            fields: self.fields,
        })
    }
}
