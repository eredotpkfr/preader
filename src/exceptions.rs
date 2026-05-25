use pyo3::{PyErr, create_exception, exceptions::PyException};

create_exception!(preader, PReaderStateError, PyException);

impl PReaderStateError {
    pub fn from_msg(msg: impl Into<String>) -> PyErr {
        Self::new_err(msg.into())
    }

    pub fn size_mismatch(saved: u64, current: u64) -> PyErr {
        Self::new_err(format!(
            "file size mismatch (saved: {saved} bytes, current: {current} bytes)"
        ))
    }

    pub fn mtime_mismatch(saved: u128, current: u128) -> PyErr {
        Self::new_err(format!(
            "file mtime mismatch (saved: {saved} ms, current: {current} ms)"
        ))
    }

    pub fn hash_mismatch(saved: &str, current: &str) -> PyErr {
        Self::new_err(format!(
            "file content hash mismatch (saved: {saved}, current: {current})"
        ))
    }

    pub fn checksum_mismatch(saved: &str, computed: &str) -> PyErr {
        Self::new_err(format!(
            "state file checksum mismatch (saved: {saved}, computed: {computed}) — state may be corrupted"
        ))
    }

    pub fn file_not_found(name: &str) -> PyErr {
        Self::new_err(format!("state file not found: {name}"))
    }
}
