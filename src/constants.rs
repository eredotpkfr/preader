use std::num::NonZeroU64;

// Read buffer size in bytes, eight times the std::io::BufReader default to amortise syscalls
pub const DEFAULT_BUFFER_CAPACITY: usize = 64 * 1024;
// Bytes of progress between two automatic state writes while auto_save_state is active
pub const DEFAULT_AUTO_SAVE_STATE_BYTES: NonZeroU64 = NonZeroU64::new(100 * 1024 * 1024).unwrap();
// Whether a loaded or resumed state is checked against the file it tracks
pub const DEFAULT_VERIFY_STATE: bool = true;
// Bytes a chunk iterator yields per item while no size is set
pub const DEFAULT_CHUNK_SIZE: usize = 1024;
// Bytes read from the start of a file to compute its identity fingerprint
pub const FINGERPRINT_SAMPLE_BYTES: u64 = 4096;
// Byte the segment iterator splits on while no delimiter is set
pub const DEFAULT_DELIMITER: u8 = b',';
// Path that means the current directory, which an empty state_dir resolves to
pub const CURRENT_DIR: &str = ".";
// Directory name appended to the platform cache directory to hold state files
pub const STATE_DIR_NAME: &str = "preader";
// Extension appended to a state name to form its file name
pub const STATE_FILE_EXTENSION: &str = "state.json";
// Extension marking the temporary file a state is written to before it is renamed into place
pub const TMP_FILE_EXTENSION: &str = "tmp";
// Separator between the directories of a state name on every platform, as in Go's io/fs
pub const NAME_SEPARATOR: &str = "/";
