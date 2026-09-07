use std::path::PathBuf;

use crate::{
    ByteBuilder, ChunkBuilder, Config, DelimiterBuilder, LineBuilder, StateRegistry,
    bases::builder::PReaderIteratorBuilder,
};

#[derive(Debug)]
pub struct PReader {
    config: Config,
    registry: StateRegistry,
}

impl Default for PReader {
    fn default() -> Self {
        Self::from(Config::default())
    }
}

impl From<Config> for PReader {
    fn from(config: Config) -> Self {
        Self {
            registry: (&config).into(),
            config,
        }
    }
}

impl PReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn states(&self) -> &StateRegistry {
        &self.registry
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

    fn builder<I: Default>(&self, file: impl Into<PathBuf>) -> PReaderIteratorBuilder<'_, I> {
        PReaderIteratorBuilder::new(&self.config, self.registry.manager(), file)
    }
}
