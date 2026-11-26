//! A file reader that tracks read percentage and manages read state
//!
//! A [`PReader`] reads a file as bytes, fixed-size chunks, UTF-8 lines or delimiter-separated
//! segments through [`PReader::bytes`], [`PReader::chunks`], [`PReader::lines`] and
//! [`PReader::segments`]. Every iterator carries a [`State`] that records its position and the
//! file it tracks; saving that state lets a later iterator continue where the previous one
//! stopped. An item counts as finished once the next one is requested, so a resumed iterator
//! repeats the item that was unfinished when the state was saved
//!
//! States are JSON files in [`Config::state_dir`] and are managed through the [`StateRegistry`]
//! returned by [`PReader::states`]. A resumed state that tracks another path fails with an
//! [`Error::Mismatch`], and with [`Config::verify_state`] on (the default) so does one whose file
//! changed. [`StateRegistry::resync`] accepts a moved or changed file; with verification on, its
//! fingerprint must still match. Reads stop at the file size recorded in the state, so data
//! appended to the file is read only after a resync, even with verification off
//!
//! # Examples
//!
//! ```
//! use preader::{Config, PReader};
//!
//! let dir = tempfile::tempdir()?;
//! let file = dir.path().join("data.txt");
//!
//! let config = Config {
//!     state_dir: dir.path().join("states"),
//!     ..Config::default()
//! };
//! let reader = PReader::from(config);
//!
//! std::fs::write(&file, "foo\nbar\nbaz\n")?;
//!
//! let mut lines = reader.lines(&file).state("job-1").build()?;
//!
//! lines.next().transpose()?;
//! lines.next().transpose()?;
//! lines.save()?;
//!
//! let resumed = reader.lines(&file).state("job-1").build()?;
//! let read = resumed.collect::<preader::Result<Vec<_>>>()?;
//!
//! assert_eq!(read, ["bar", "baz"]);
//! # Ok::<(), preader::Error>(())
//! ```

#![forbid(unsafe_code)]
#![doc(test(attr(deny(warnings))))]
#![warn(
    missing_docs,
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

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
#[doc(hidden)]
pub use {
    constants::TMP_FILE_EXTENSION,
    enums::{autosave::AutoSave, skip::SkipPlan},
    types::window::Window,
    utils::{file, hash, path, time},
    validators::validate_name,
};
