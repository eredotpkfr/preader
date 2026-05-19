use std::{fs, path::PathBuf};

use anyhow::Context;
use pyo3::{prelude::*, types::PyType};
use serde::Deserialize;

use crate::{enums::PReaderConfigFormat, exceptions::PReaderConfigError};

// std::io::BufReader default = 8 KiB; we use 64 KiB to amortize syscalls on
// large files where preader is typically used
const DEFAULT_BUFFER_CAPACITY: usize = 64 * 1024;

#[pyclass(from_py_object)]
#[derive(Clone, Deserialize)]
pub struct PReaderConfig {
    #[pyo3(get)]
    pub buffer_capacity: usize,
}

impl Default for PReaderConfig {
    fn default() -> Self {
        Self {
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
        }
    }
}

#[pymethods]
impl PReaderConfig {
    #[new]
    #[pyo3(signature = (buffer_capacity = DEFAULT_BUFFER_CAPACITY))]
    pub fn new(buffer_capacity: usize) -> Self {
        Self { buffer_capacity }
    }

    #[classmethod]
    #[pyo3(signature = (path, format=PReaderConfigFormat::Json))]
    fn load(
        _cls: &Bound<'_, PyType>,
        path: PathBuf,
        format: PReaderConfigFormat,
    ) -> PyResult<Self> {
        fs::read_to_string(&path)
            .context("failed to read config file")
            .and_then(|content| format.parse(&content))
            .map_err(PReaderConfigError::from_anyhow)
    }
}
