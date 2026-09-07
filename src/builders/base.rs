use crate::{IteratorOptions, PReaderIteratorBuilder, StateInput};

impl<I> PReaderIteratorBuilder<'_, I> {
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
