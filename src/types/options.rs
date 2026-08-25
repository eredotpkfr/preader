use pyo3::{exceptions::PyValueError, prelude::*};

use crate::types::window::Window;

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct IteratorOptions {
    #[pyo3(get, set)]
    pub start: u64,
    #[pyo3(get, set)]
    pub end: u64,
    #[pyo3(get, set)]
    pub skip: u64,
    #[pyo3(get, set)]
    pub limit: u64,
}

impl Default for IteratorOptions {
    fn default() -> Self {
        Self {
            start: 0,
            end: u64::MAX,
            skip: 0,
            limit: u64::MAX,
        }
    }
}

impl IteratorOptions {
    pub(crate) fn validate(&self) -> PyResult<()> {
        if self.start > self.end {
            return Err(PyValueError::new_err(format!(
                "start ({}) must be <= end ({})",
                self.start, self.end
            )));
        }

        Ok(())
    }

    pub fn window(&self, position: u64, size: u64, skip_bytes: u64) -> Window {
        let end = self.end.min(size);
        let start = self.start.saturating_add(skip_bytes).min(end);

        Window {
            position: position.max(start),
            end,
            from_start: position <= start && position < end,
        }
    }
}

#[pymethods]
impl IteratorOptions {
    #[new]
    #[pyo3(signature = (
        *,
        start = 0,
        end = u64::MAX,
        skip = 0,
        limit = u64::MAX,
    ))]
    pub fn new(start: u64, end: u64, skip: u64, limit: u64) -> Self {
        Self {
            start,
            end,
            skip,
            limit,
        }
    }

    fn __repr__(&self) -> String {
        crate::macros::pyrepr!("IteratorOptions" {
            start = self.start,
            end = self.end,
            skip = self.skip,
            limit = self.limit,
        })
    }
}
