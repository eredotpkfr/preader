use pyo3::prelude::*;

mod exceptions;
mod iterators;
mod reader;
mod types;
mod utils;

pub use exceptions::PReaderStateError;
pub use iterators::PReaderIteratorBase;
pub use reader::PReader;
pub use types::{
    FileMetadata, PReaderState, PReaderStateManager, Timestamps, config::PReaderConfig,
};

#[pymodule]
mod preader {
    #[pymodule_export]
    use super::{
        FileMetadata, PReader, PReaderConfig, PReaderIteratorBase, PReaderState, PReaderStateError,
        Timestamps,
    };
}
