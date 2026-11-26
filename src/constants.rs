use std::num::NonZeroU64;

/// Default [`buffer_capacity`](crate::Config::buffer_capacity) in bytes
///
/// Eight times the [`std::io::BufReader`] default, to amortize syscalls
pub const DEFAULT_BUFFER_CAPACITY: usize = 64 * 1024;
/// Default [`auto_save_state_bytes`](crate::Config::auto_save_state_bytes), 100 MiB
pub const DEFAULT_AUTO_SAVE_STATE_BYTES: NonZeroU64 = NonZeroU64::new(100 * 1024 * 1024).unwrap();
/// Default [`verify_state`](crate::Config::verify_state), so resumed and loaded states are verified
pub const DEFAULT_VERIFY_STATE: bool = true;
/// Default chunk size in bytes, changed with [`ChunkBuilder::size`](crate::ChunkBuilder::size)
pub const DEFAULT_CHUNK_SIZE: usize = 1024;
/// Bytes read from the start of a file for its [`fingerprint`](crate::FileMetadata::fingerprint)
///
/// A change past this prefix is noticed only through the file size or modification time
pub const FINGERPRINT_SAMPLE_BYTES: u64 = 4096;
/// Default segment delimiter, changed with [`delimiter`](crate::SegmentBuilder::delimiter)
pub const DEFAULT_DELIMITER: u8 = b',';
/// Current directory, which an empty [`state_dir`](crate::Config::state_dir) resolves to
pub const CURRENT_DIR: &str = ".";
/// Name of the default [`state_dir`](crate::Config::state_dir) inside the user's cache directory
///
/// Without a cache directory it is used as a relative path, see
/// [`Config::default`](crate::Config::default)
pub const STATE_DIR_NAME: &str = "preader";
/// Extension appended to a state name to form its file name
///
/// A state named `a/b` is saved as `a/b.state.json` inside the state directory
pub const STATE_FILE_EXTENSION: &str = "state.json";
/// Extension of the temporary file a state is written to before it is renamed into place
pub const TMP_FILE_EXTENSION: &str = "tmp";
/// Separator between the parts of a state name on every platform, as in Go's `io/fs`
pub const NAME_SEPARATOR: &str = "/";
