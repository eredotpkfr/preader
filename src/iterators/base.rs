use std::fs::File;

use pyo3::prelude::*;

#[pyclass(subclass)]
pub struct PReaderBaseFileIterator {
    #[pyo3(get)]
    pub bytes_read: usize,
    #[pyo3(get)]
    pub total_bytes: usize,
}

impl TryFrom<&File> for PReaderBaseFileIterator {
    type Error = std::io::Error;

    fn try_from(file: &File) -> std::io::Result<Self> {
        Ok(Self {
            bytes_read: 0,
            total_bytes: file.metadata()?.len() as usize,
        })
    }
}

#[pymethods]
impl PReaderBaseFileIterator {
    fn percentage(&self) -> f64 {
        self.bytes_read as f64 * 100.0 / self.total_bytes.max(1) as f64
    }
}
