use pyo3::{prelude::*, types::PyBytes};

use crate::python::macros::iterator::pyiterator;

pyiterator!(
    /// Reads a file in fixed-size chunks, yielding each chunk as `bytes`
    ///
    /// Create one with `reader.chunks(file, chunk_size=1024)`. The last chunk is shorter when the
    /// read range does not divide evenly, and `drop_partial` drops it
    ChunkIterator(crate::ChunkIterator) -> Py<PyBytes>,
    |py, item| PyBytes::new(py, item).unbind(),
    [chunk_size, drop_partial]

    /// Size in bytes of every chunk but the last; 0 yields no chunks
    #[getter]
    fn chunk_size(&self) -> usize {
        self.0.inner.size
    }

    /// Whether a last chunk shorter than `chunk_size` is dropped
    #[getter]
    fn drop_partial(&self) -> bool {
        self.0.inner.drop_partial
    }
);
