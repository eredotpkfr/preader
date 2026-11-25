use pyo3::{prelude::*, types::PyBytes};

use crate::python::macros::iterator::pyiterator;

pyiterator!(
    /// Reads a file split on a one-byte delimiter, yielding each segment as `bytes`
    ///
    /// Create one with `reader.segments(file, delimiter=b",")`. One trailing delimiter is removed
    /// from each segment unless `keep_delimiter` is set. A last segment without the delimiter is
    /// not yielded when the file has grown past its recorded size, since its end is not known yet
    SegmentIterator(crate::SegmentIterator) -> Py<PyBytes>,
    |py, item| PyBytes::new(py, item).unbind(),
    [delimiter, keep_delimiter, skip_empty, skip_remaining]

    /// One-byte delimiter that ends each segment
    #[getter]
    fn delimiter(&self) -> [u8; 1] {
        [self.0.inner.delimiter]
    }

    /// Whether each segment keeps its trailing delimiter
    #[getter]
    fn keep_delimiter(&self) -> bool {
        self.0.inner.keep
    }

    /// Whether segments that are empty once the delimiter is removed are skipped
    #[getter]
    fn skip_empty(&self) -> bool {
        self.0.inner.skip_empty
    }

    /// Number of segments still to be skipped before the next one is yielded
    ///
    /// With `align_start`, it includes the partial segment skipped at `options.start`
    #[getter]
    fn skip_remaining(&self) -> u64 {
        self.0.progress.skipping
    }
);
