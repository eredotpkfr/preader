use pyo3::prelude::*;

mod config;
mod enums;
mod exceptions;
mod iterators;
mod reader;

pub use config::PReaderConfig;
pub use enums::PReaderConfigFormat;
pub use exceptions::PReaderConfigError;
pub use iterators::{
    PReaderBaseFileIterator, PReaderByteIterator, PReaderChunkIterator, PReaderDelimiterIterator,
    PReaderLineIterator,
};
pub use reader::PReader;

/// A Python package for reading files with live read-percentage tracking
#[pymodule]
mod preader {
    #[pymodule_export]
    use super::{
        PReader, PReaderBaseFileIterator, PReaderConfig, PReaderConfigError, PReaderConfigFormat,
    };
}
