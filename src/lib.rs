use pyo3::prelude::*;

mod config;
mod enums;
mod exceptions;

pub use config::PReaderConfig;
pub use enums::PReaderConfigFormat;
pub use exceptions::PReaderConfigError;

/// A Python package for reading files with live read-percentage tracking
#[pymodule]
mod preader {
    #[pymodule_export]
    use super::{PReaderConfig, PReaderConfigError, PReaderConfigFormat};
}
