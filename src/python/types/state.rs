use std::path::PathBuf;

use pyo3::prelude::*;

use crate::{
    FileMetadata, State, Timestamps,
    python::{macros::repr::pyrepr, utils::text::quote},
};

#[pymethods]
impl State {
    #[getter]
    fn name(&self) -> &str {
        &self.data.name
    }

    #[getter]
    fn file(&self) -> FileMetadata {
        self.data.file.clone()
    }

    #[getter]
    fn position(&self) -> u64 {
        self.data.position
    }

    #[getter]
    fn timestamps(&self) -> Timestamps {
        self.data.timestamps.clone()
    }

    #[pyo3(name = "path")]
    fn py_path(&self) -> PyResult<PathBuf> {
        Ok(self.path()?)
    }

    #[pyo3(name = "checksum")]
    fn py_checksum(&self) -> PyResult<String> {
        Ok(self.checksum()?)
    }

    #[pyo3(name = "percent")]
    fn py_percent(&self) -> f64 {
        self.percent()
    }

    #[pyo3(name = "verify")]
    fn py_verify(&self) -> PyResult<()> {
        Ok(self.verify()?)
    }

    #[pyo3(name = "save")]
    fn py_save(&mut self) -> PyResult<PathBuf> {
        Ok(self.save()?)
    }

    #[pyo3(name = "resync")]
    fn py_resync(&self, path: PathBuf) -> PyResult<Self> {
        Ok(self.resync(path)?)
    }

    pub(crate) fn __repr__(&self) -> String {
        pyrepr!("State" {
            name = quote(&self.data.name),
            file = self.file().__repr__(),
            position = self.data.position,
            timestamps = self.timestamps().__repr__(),
        })
    }
}
