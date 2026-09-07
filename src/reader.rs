use std::path::PathBuf;

use derive_more::From;

use crate::{
    ByteBuilder, ChunkBuilder, Config, DelimiterBuilder, LineBuilder, StateRegistry,
    bases::builder::PReaderIteratorBuilder,
};

#[derive(Debug, Default, From)]
pub struct PReader {
    pub config: Config,
}

impl PReader {
    pub fn new() -> Self {
        Self::default()
    }

    fn builder<I: Default>(&self, file: impl Into<PathBuf>) -> PReaderIteratorBuilder<'_, I> {
        PReaderIteratorBuilder::new(&self.config, file)
    }

    pub fn states(&self) -> StateRegistry {
        StateRegistry::from(&self.config)
    }

    pub fn bytes(&self, file: impl Into<PathBuf>) -> ByteBuilder<'_> {
        self.builder(file)
    }

    pub fn chunks(&self, file: impl Into<PathBuf>) -> ChunkBuilder<'_> {
        self.builder(file)
    }

    pub fn lines(&self, file: impl Into<PathBuf>) -> LineBuilder<'_> {
        self.builder(file)
    }

    pub fn delimiter(&self, file: impl Into<PathBuf>) -> DelimiterBuilder<'_> {
        self.builder(file)
    }
}
