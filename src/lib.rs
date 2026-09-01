use pyo3::prelude::*;

mod macros;

mod enums;
mod exceptions;
mod iterators;
mod manager;
mod reader;
mod registry;
mod types;
mod utils;

pub use enums::error::Error;
pub use exceptions::StateError;
pub use iterators::{
    base::IteratorBase, byte::ByteIterator, chunk::ChunkIterator, delimiter::DelimiterIterator,
    line::LineIterator, state::StateIterator,
};
pub use manager::{STATE_FILE_SUFFIX, StateManager, TMP_STATE_FILE_SUFFIX};
pub use reader::PReader;
pub use registry::StateRegistry;
pub use types::{
    checksum::ChecksumBody,
    config::{
        iterator::IteratorConfig,
        manager::StateManagerConfig,
        reader::{Config, DEFAULT_VERIFY_STATE},
    },
    file::FileMetadata,
    options::IteratorOptions,
    state::{State, StateData},
    time::Timestamps,
    window::Window,
};
pub use utils::{
    file::{fingerprint, starts_mid_item},
    path::{
        DEFAULT_STATE_DIRECTORY, default_state_dir, has_no_symlinks, normalize_path, path_stem,
        scoped_join, strip_extensions,
    },
    text::indent_lines,
};

#[pymodule]
mod preader {
    #[pymodule_export]
    use super::{
        ByteIterator, ChunkIterator, Config, DelimiterIterator, FileMetadata, IteratorBase,
        IteratorOptions, LineIterator, PReader, State, StateError, StateIterator, StateRegistry,
        Timestamps,
    };
}
