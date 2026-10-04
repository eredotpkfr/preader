use pyo3::prelude::*;

use crate::{IteratorOptions, python::macros::repr::pyrepr};

#[pymethods]
impl IteratorOptions {
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

    fn __repr__(&self) -> String {
        pyrepr!("IteratorOptions" {
            start = self.start,
            end = self.end,
            skip = self.skip,
            limit = self.limit,
        })
    }
}
