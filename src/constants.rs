// Read buffer size in bytes, eight times the std::io::BufReader default to amortise syscalls
pub const DEFAULT_BUFFER_CAPACITY: usize = 64 * 1024;
// Bytes of progress between two automatic state writes while auto_save_state is active
pub const DEFAULT_AUTO_SAVE_STATE_BYTES: u64 = 100 * 1024 * 1024;
// Whether a loaded or resumed state is checked against the file it tracks
pub const DEFAULT_VERIFY_STATE: bool = true;
// Bytes a chunk iterator yields per item while no size is set
pub const DEFAULT_CHUNK_SIZE: usize = 1024;
// Bytes read from the start of a file to compute its identity fingerprint
pub const FINGERPRINT_SAMPLE_BYTES: u64 = 4096;
// Byte a delimiter iterator splits on while no character is set
pub const DEFAULT_DELIMITER: u8 = b',';
// Directory name appended to the platform cache directory to hold state files
pub const DEFAULT_STATE_DIR: &str = "preader";
// Extension appended to a state name to form its file name, stored without a leading dot
pub const STATE_FILE_EXTENSION: &str = "state.json";
// Extension marking the temporary file a state is written to before it is renamed into place
pub const TMP_FILE_EXTENSION: &str = "tmp";
