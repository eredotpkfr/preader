use pyo3::prelude::*;

mod enums;
mod exceptions;
mod iterators;
mod reader;
mod types;
mod utils;

pub use exceptions::StateError;
pub use iterators::IteratorBase;
pub use reader::PReader;
pub use types::{FileMetadata, IteratorOptions, State, StateManager, Timestamps, config::Config};

#[pymodule]
mod preader {
    #[pymodule_export]
    use super::{Config, IteratorOptions, PReader, State, StateError};
}
