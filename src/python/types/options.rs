use pyo3::prelude::*;

use crate::{IteratorOptions, python::macros::repr::pyrepr};

#[pymethods]
impl IteratorOptions {
    /// Creates iterator options from keyword arguments; omitted ones read the whole file
    ///
    /// `start` and `end` are byte offsets and `limit` is the most items one iterator yields.
    /// `skip` moves `start` forward by that many bytes or chunks, while lines and segments are
    /// skipped only when a resumed state is not past `start`. `start` greater than `end` raises
    /// `ValueError` when the iterator is created, not here
    #[new]
    #[pyo3(signature = (*, start = 0, end = u64::MAX, skip = 0, limit = u64::MAX))]
    fn py_new(start: u64, end: u64, skip: u64, limit: u64) -> Self {
        Self {
            start,
            end,
            skip,
            limit,
        }
    }

    /// Returns the class name with every option
    fn __repr__(slf: &Bound<'_, Self>) -> PyResult<String> {
        pyrepr!(slf, start, end, skip, limit)
    }
}
