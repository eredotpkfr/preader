use pyo3::{prelude::*, types::PyString};

use crate::python::macros::iterator::pyiterator;

pyiterator!(
    /// Reads a UTF-8 file line by line, yielding each line as `str`
    ///
    /// Create one with `reader.lines(file)`. Only a newline ends a line; the whole trailing run of
    /// newline and carriage return characters is removed unless `keepends` is set. A last line
    /// without a newline is not yielded when the file has grown past its recorded size, since
    /// its end is not known yet
    LineIterator(crate::LineIterator) -> Py<PyString>,
    |py, item| PyString::new(py, item).unbind(),
    [keepends, skip_empty, skip_remaining]

    /// Whether each line keeps its trailing newline and carriage return characters
    #[getter]
    fn keepends(&self) -> bool {
        self.0.inner.keepends
    }

    /// Whether lines that are empty once the terminator is removed are skipped
    #[getter]
    fn skip_empty(&self) -> bool {
        self.0.inner.skip_empty
    }

    /// Number of lines still to be skipped before the next one is yielded
    ///
    /// With `align_start`, it includes the partial line skipped at `options.start`
    #[getter]
    fn skip_remaining(&self) -> u64 {
        self.0.progress.skipping
    }
);
