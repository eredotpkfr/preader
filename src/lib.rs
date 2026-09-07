#![forbid(unsafe_code)]

mod bases;
mod builders;
mod enums;
mod interfaces;
mod iterators;
mod manager;
mod reader;
mod registry;
mod types;
mod utils;

pub use bases::{builder::PReaderIteratorBuilder, iterator::PReaderIterator};
pub use enums::{
    autosave::AutoSave,
    error::{core::Error, mismatch::Mismatch, path::PathError},
    input::StateInput,
    skip::Skip,
};
pub use interfaces::{builder::IteratorBuild, iterator::IteratorRead};
pub use iterators::{
    byte::Byte,
    chunk::{Chunk, DEFAULT_CHUNK_SIZE},
    delimiter::{DEFAULT_DELIMITER, Delimiter},
    line::Line,
};
pub use manager::{STATE_FILE_EXTENSION, StateManager, TMP_FILE_EXTENSION};
pub use reader::PReader;
pub use registry::{StateIterator, StateRegistry};
pub use types::{
    checksum::ChecksumBody,
    config::{
        manager::StateManagerConfig,
        reader::{
            Config, DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, DEFAULT_VERIFY_STATE,
        },
    },
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
        DEFAULT_STATE_DIR, default_state_dir, has_no_symlinks, normalize_path, path_stem,
        scoped_join, strip_extensions,
    },
};
