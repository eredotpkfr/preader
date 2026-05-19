use std::{fs, path::PathBuf};

use anyhow::Context;
use pyo3::{prelude::*, types::PyType};
use serde::Deserialize;

use crate::{enums::PReaderConfigFormat, exceptions::PReaderConfigError};

// std::io::BufReader default = 8 KiB; we use 64 KiB to amortize syscalls on
// large files where preader is typically used
const DEFAULT_BUFFER_CAPACITY: usize = 64 * 1024;
const DEFAULT_NEWLINE_DELIMITER: [u8; 1] = *b"\n";

#[pyclass]
#[derive(Deserialize)]
pub struct PReaderConfig {
    #[pyo3(get)]
    buffer_capacity: usize,
    #[pyo3(get)]
    newline_delimiter: Vec<u8>,
}

#[pymethods]
impl PReaderConfig {
    #[new]
    #[pyo3(signature = (buffer_capacity=None, newline_delimiter=None))]
    fn new(buffer_capacity: Option<usize>, newline_delimiter: Option<Vec<u8>>) -> Self {
        Self {
            buffer_capacity: buffer_capacity.unwrap_or(DEFAULT_BUFFER_CAPACITY),
            newline_delimiter: newline_delimiter.unwrap_or(DEFAULT_NEWLINE_DELIMITER.into()),
        }
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
