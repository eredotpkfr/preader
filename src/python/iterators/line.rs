use pyo3::{prelude::*, types::PyString};

use crate::python::macros::iterator::pyiterator;

pyiterator!(
    LineIterator(crate::LineIterator) as "LineIterator" -> Py<PyString>,
    |py, item| PyString::new(py, item).unbind(),
    [keepends, skip_empty, skip_remaining]

    #[getter]
    fn keepends(&self) -> bool {
        self.0.inner.keepends
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
