#![forbid(unsafe_code)]

mod bases;
mod builders;
mod constants;
mod enums;
mod interfaces;
mod iterators;
mod macros;
mod manager;
mod preader;
#[cfg(feature = "python")]
mod python;
mod registry;
mod types;
mod utils;
mod validators;

pub use bases::{builder::PReaderIteratorBuilder, iterator::PReaderIterator};
pub use constants::{
    DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, DEFAULT_CHUNK_SIZE, DEFAULT_DELIMITER,
    DEFAULT_STATE_DIR, DEFAULT_VERIFY_STATE, FINGERPRINT_SAMPLE_BYTES, STATE_FILE_EXTENSION,
};
pub use enums::{
    error::{core::Error, mismatch::Mismatch, name::NameError},
    source::StateSource,
};
pub use interfaces::{builder::IteratorBuild, iterator::IteratorRead};
pub use iterators::{
    byte::Byte, chunk::Chunk, delimiter::Delimiter, line::Line, state::StateIterator,
};
pub use preader::PReader;
pub use registry::StateRegistry;
pub(crate) use types::progress::Progress;
pub use types::{
    config::Config,
    core::{
        ByteBuilder, ByteIterator, ChunkBuilder, ChunkIterator, DelimiterBuilder,
        DelimiterIterator, LineBuilder, LineIterator, Result,
    },
    file::FileMetadata,
    options::IteratorOptions,
    state::{State, StateData},
    time::Timestamps,
};
#[cfg(feature = "testing")]
pub use {
    constants::TMP_FILE_EXTENSION,
    enums::{autosave::AutoSave, skip::Skip},
    manager::StateManager,
    types::{checksum::ChecksumBody, window::Window},
    utils::{
        file::{fingerprint, starts_mid_item},
        path::{default_state_dir, resolves_in_place, scoped_join},
    },
    validators::validate_name,
};
