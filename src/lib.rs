#![forbid(unsafe_code)]

mod bases;
mod builders;
mod constants;
mod enums;
mod interfaces;
mod iterators;
mod manager;
mod preader;
mod registry;
mod types;
mod utils;

pub use bases::{builder::PReaderIteratorBuilder, iterator::PReaderIterator};
pub use constants::{
    DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, DEFAULT_CHUNK_SIZE, DEFAULT_DELIMITER,
    DEFAULT_STATE_DIR, DEFAULT_VERIFY_STATE, STATE_FILE_EXTENSION, TMP_FILE_EXTENSION,
};
pub use enums::{
    autosave::AutoSave,
    error::{core::Error, mismatch::Mismatch, path::PathError},
    input::StateInput,
    skip::Skip,
};
pub use interfaces::{builder::IteratorBuild, iterator::IteratorRead};
pub use iterators::{
    byte::Byte, chunk::Chunk, delimiter::Delimiter, line::Line, state::StateIterator,
};
pub use manager::StateManager;
pub use preader::PReader;
pub use registry::StateRegistry;
pub use types::{
    checksum::ChecksumBody,
    config::Config,
    core::{
        ByteBuilder, ByteIterator, ChunkBuilder, ChunkIterator, DelimiterBuilder,
        DelimiterIterator, LineBuilder, LineIterator, Result,
    },
    file::FileMetadata,
    options::IteratorOptions,
    progress::Progress,
    state::{State, StateData},
    time::Timestamps,
    window::Window,
};
pub use utils::{
    file::{fingerprint, starts_mid_item},
    path::{
        default_state_dir, has_no_symlinks, normalize_path, path_stem, scoped_join,
        strip_extensions,
    },
};
