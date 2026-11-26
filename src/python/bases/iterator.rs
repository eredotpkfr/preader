use pyo3::prelude::*;

/// Common base class of `ByteIterator`, `ChunkIterator`, `LineIterator` and `SegmentIterator`
///
/// It has no members of its own; use it with `isinstance` to accept any preader file iterator
#[pyclass(module = "preader", subclass)]
pub struct IteratorBase;
