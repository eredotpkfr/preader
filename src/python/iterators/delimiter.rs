use pyo3::{prelude::*, types::PyBytes};

use crate::python::macros::iterator::pyiterator;

pyiterator!(
    DelimiterIterator(crate::DelimiterIterator) as "DelimiterIterator" -> Py<PyBytes>,
    |py, item| PyBytes::new(py, item).unbind(),
    [delimiter, keep_delimiter, skip_empty, skip_remaining]

    #[getter]
    fn delimiter(&self) -> u8 {
        self.0.inner.character
    }

    #[getter]
    fn keep_delimiter(&self) -> bool {
        self.0.inner.keep
    }

    #[getter]
    fn skip_empty(&self) -> bool {
        self.0.inner.skip_empty
    }

    #[getter]
    fn skip_remaining(&self) -> u64 {
        self.0.progress.skipping
    }
);
