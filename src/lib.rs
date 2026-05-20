use pyo3::prelude::*;

mod enums;
mod exceptions;
mod iterators;
mod reader;
mod types;

pub use enums::PReaderConfigFormat;
pub use exceptions::PReaderConfigError;
pub use reader::PReader;
pub use types::{PReaderConfig, PReaderItem};

/// A Python package for reading files with live read-percentage tracking
#[pymodule]
mod preader {
    #[pymodule_export]
    use super::{PReader, PReaderConfig, PReaderConfigError, PReaderConfigFormat, PReaderItem};
}
