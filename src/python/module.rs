/// A file reader that tracks read percentage and manages read state
#[pyo3::pymodule]
mod preader {
    /// The classes and exceptions the `preader` module exports
    #[pymodule_export]
    use crate::{
        Config, FileMetadata, IteratorOptions, PReader, State, StateIterator, StateRegistry,
        Timestamps,
        python::{
            bases::iterator::IteratorBase,
            exceptions::{
                StateConflictError, StateCorruptError, StateError, StateMismatchError,
                StateOutdatedError,
            },
            iterators::{
                byte::ByteIterator, chunk::ChunkIterator, line::LineIterator,
                segment::SegmentIterator,
            },
        },
    };
}
