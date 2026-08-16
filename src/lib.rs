use pyo3::prelude::*;

mod macros;

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
    base::IteratorBase, byte::ByteIterator, chunk::ChunkIterator, delimiter::DelimiterIterator,
    line::LineIterator, state::StateIterator,
};
pub use manager::StateManager;
pub use reader::PReader;
pub use registry::StateRegistry;
pub use types::{
    config::reader::Config, file::FileMetadata, options::IteratorOptions, state::State,
    time::Timestamps,
};

#[pymodule]
mod preader {
    #[pymodule_export]
    use super::{
        ByteIterator, ChunkIterator, Config, DelimiterIterator, FileMetadata, IteratorBase,
        IteratorOptions, LineIterator, PReader, State, StateError, StateIterator, StateRegistry,
        Timestamps,
    };
}
