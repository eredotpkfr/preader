use pyo3::{prelude::*, types::PyBytes};

use crate::python::macros::iterator::pyiterator;

pyiterator!(
    /// Reads a file one byte at a time, yielding each byte as a one-byte `bytes` object
    ///
    /// Create one with `reader.bytes(file)`
    ByteIterator(crate::ByteIterator) -> Py<PyBytes>,
    |py, item| PyBytes::new(py, &[item]).unbind(),
    []
);
