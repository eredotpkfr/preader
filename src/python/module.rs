/// A Python package that reads a file along with its read percentage.
#[pyo3::pymodule]
mod preader {
    #[pymodule_export]
    use crate::{
        Config, FileMetadata, IteratorOptions, PReader, State, StateIterator, StateRegistry,
        Timestamps,
        python::{
            bases::iterator::IteratorBase,
            exceptions::{StateError, StateMismatchError},
            iterators::{
                byte::ByteIterator, chunk::ChunkIterator, delimiter::DelimiterIterator,
                line::LineIterator,
            },
        },
    };
}
