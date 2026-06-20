use pyo3::prelude::*;

mod enums;
mod exceptions;
mod iterators;
mod manager;
mod reader;
mod registry;
mod types;
mod utils;

pub use exceptions::StateError;
pub use iterators::{
    ByteIterator, ChunkIterator, DelimiterIterator, IteratorBase, LineIterator, StateIterator,
};
pub use manager::StateManager;
pub use reader::PReader;
pub use registry::StateRegistry;
pub use types::{FileMetadata, IteratorOptions, State, Timestamps, config::Config};

#[pymodule]
mod preader {
    #[pymodule_export]
    use super::{
        ByteIterator, ChunkIterator, Config, DelimiterIterator, FileMetadata, IteratorBase,
        IteratorOptions, LineIterator, PReader, State, StateError, StateIterator, StateRegistry,
        Timestamps,
    };
}
