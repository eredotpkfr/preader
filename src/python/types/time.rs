use pyo3::prelude::*;

use crate::{Timestamps, python::macros::repr::pyrepr};

#[pymethods]
impl Timestamps {
    pub(crate) fn __repr__(&self) -> String {
        pyrepr!("Timestamps" {
            created_at = self.created_at.timestamp(),
            updated_at = self.updated_at.timestamp(),
        })
    }
}
