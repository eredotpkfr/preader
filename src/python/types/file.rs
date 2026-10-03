use pyo3::prelude::*;

use crate::{
    FileMetadata,
    python::{macros::repr::pyrepr, utils::text::quote},
};

#[pymethods]
impl FileMetadata {
    pub(crate) fn __repr__(&self) -> String {
        pyrepr!("FileMetadata" {
            path = quote(self.path.display()),
            size = self.size,
            mtime = self.mtime.timestamp(),
            fingerprint = quote(&self.fingerprint),
        })
    }
}
