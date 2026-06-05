use pyo3::prelude::*;

mod enums;
mod exceptions;
mod iterators;
mod manager;
mod reader;
mod registry;
mod types;
mod utils;

pub use exceptions::StateError;
pub use iterators::IteratorBase;
pub use manager::StateManager;
pub use reader::PReader;
pub use registry::StateRegistry;
pub use types::{FileMetadata, IteratorOptions, State, Timestamps, config::Config};

#[pymodule]
mod preader {
    #[pymodule_export]
    use super::{Config, IteratorOptions, PReader, State, StateError};
}
