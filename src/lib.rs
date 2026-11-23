#![forbid(unsafe_code)]

mod bases;
mod builders;
mod constants;
mod enums;
mod interfaces;
mod iterators;
mod macros;
mod preader;
#[cfg(feature = "python")]
mod python;
mod registry;
mod serializers;
mod types;
mod utils;
mod validators;

pub use bases::{builder::PReaderIteratorBuilder, iterator::PReaderIterator};
pub use constants::{
    DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, DEFAULT_CHUNK_SIZE, DEFAULT_DELIMITER,
    DEFAULT_VERIFY_STATE, FINGERPRINT_SAMPLE_BYTES, STATE_DIR_NAME, STATE_FILE_EXTENSION,
};
pub use enums::{
    error::{core::Error, mismatch::Mismatch, name::NameError},
    source::StateSource,
};
pub use interfaces::iterator::IteratorRead;
pub use iterators::{byte::Byte, chunk::Chunk, line::Line, segment::Segment, state::StateIterator};
pub use preader::PReader;
pub use registry::StateRegistry;
pub(crate) use types::progress::Progress;
pub use types::{
    config::Config,
    core::{
        ByteBuilder, ByteIterator, ChunkBuilder, ChunkIterator, LineBuilder, LineIterator, Result,
        SegmentBuilder, SegmentIterator,
    },
    file::FileMetadata,
    options::IteratorOptions,
    state::State,
    time::Timestamps,
};
#[cfg(feature = "testing")]
pub use {
    constants::TMP_FILE_EXTENSION,
    enums::{autosave::AutoSave, skip::SkipPlan},
    types::window::Window,
    utils::{file, hash, path, time},
    validators::validate_name,
};
