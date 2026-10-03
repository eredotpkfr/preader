use pyo3::{prelude::*, types::PyBytes};

use crate::python::macros::iterator::pyiterator;

pyiterator!(
    ChunkIterator(crate::ChunkIterator) as "ChunkIterator" -> Py<PyBytes>,
    |py, item| PyBytes::new(py, item).unbind(),
    [chunk_size, drop_partial]

    #[getter]
    fn chunk_size(&self) -> usize {
        self.0.inner.size
    }

    #[getter]
    fn drop_partial(&self) -> bool {
        self.0.inner.drop_partial
    }
);
