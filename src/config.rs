use std::fs;
use std::path::PathBuf;

use pyo3::prelude::*;
use pyo3::types::PyType;
use serde::Deserialize;

use crate::enums::PReaderConfigFormat;
use crate::exceptions::PReaderConfigError;

// std::io::BufReader default = 8 KiB; we use 64 KiB to amortize syscalls on
// large files where preader is typically used
const DEFAULT_BUFFER_CAPACITY: usize = 64 * 1024;
const DEFAULT_NEWLINE_DELIMITER: u8 = b'\n';

#[pyclass]
#[derive(Deserialize)]
pub struct PReaderConfig {
    #[pyo3(get)]
    buffer_capacity: usize,
    #[pyo3(get)]
    newline_delimiter: Vec<u8>,
}

impl Default for PReaderConfig {
    fn default() -> Self {
        Self {
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
            newline_delimiter: vec![DEFAULT_NEWLINE_DELIMITER],
        }
    }
}

#[pymethods]
impl PReaderConfig {
    #[new]
    #[pyo3(signature = (buffer_capacity=None, newline_delimiter=None))]
    fn new(buffer_capacity: Option<usize>, newline_delimiter: Option<Vec<u8>>) -> Self {
        let default = Self::default();

        Self {
            buffer_capacity: buffer_capacity.unwrap_or(default.buffer_capacity),
            newline_delimiter: newline_delimiter.unwrap_or(default.newline_delimiter),
        }
    }

    #[classmethod]
    #[pyo3(signature = (path, format=PReaderConfigFormat::JSON))]
    fn load(
        _cls: &Bound<'_, PyType>,
        path: PathBuf,
        format: PReaderConfigFormat,
    ) -> PyResult<Self> {
        let content = fs::read_to_string(&path)
            .map_err(|e| PReaderConfigError::new_err(format!("failed to read {path:?}: {e}")))?;

        match format {
            PReaderConfigFormat::JSON => Self::load_json(&content),
            PReaderConfigFormat::TOML => Self::load_toml(&content),
            PReaderConfigFormat::YML => Self::load_yml(&content),
        }
        .map_err(|e| PReaderConfigError::new_err(format!("failed to parse {path:?} as {e}")))
    }
}

impl PReaderConfig {
    fn load_json(content: &str) -> Result<Self, String> {
        serde_json::from_str(content).map_err(|e| format!("JSON: {e}"))
    }

    fn load_yml(content: &str) -> Result<Self, String> {
        serde_norway::from_str(content).map_err(|e| format!("YAML: {e}"))
    }

    fn load_toml(content: &str) -> Result<Self, String> {
        toml::from_str(content).map_err(|e| format!("TOML: {e}"))
    }
}
