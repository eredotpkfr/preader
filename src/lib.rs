use pyo3::prelude::*;

mod config;
mod enums;
mod exceptions;

pub use config::PReaderConfig;
pub use enums::PReaderConfigFormat;
pub use exceptions::PReaderConfigError;

/// A Python module implemented in Rust
#[pymodule]
mod preader {
    #[pymodule_export]
    use super::{PReaderConfig, PReaderConfigError, PReaderConfigFormat};
}
