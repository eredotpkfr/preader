use pyo3::{prelude::*, types::PyBytes};

use crate::python::macros::iterator::pyiterator;

pyiterator!(
    ByteIterator(crate::ByteIterator) as "ByteIterator" -> Py<PyBytes>,
    |py, item| PyBytes::new(py, &[item]).unbind(),
    []
);
