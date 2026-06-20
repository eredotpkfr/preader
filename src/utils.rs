use std::{
    fs::File,
    io::{Read, Result},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

const DEFAULT_STATE_DIRECTORY: &str = "preader";
const FINGERPRINT_BYTES: u64 = 4096;

pub fn default_state_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_default().join(DEFAULT_STATE_DIRECTORY)
}

pub fn indent_lines(text: &str, width: usize) -> String {
    text.replace('\n', &format!("\n{}", " ".repeat(width)))
}

pub fn fingerprint(path: &Path) -> Result<String> {
    let mut buffer = Vec::with_capacity(FINGERPRINT_BYTES as usize);

    File::open(path)?.take(FINGERPRINT_BYTES).read_to_end(&mut buffer)?;

    Ok(hex::encode(Sha256::digest(&buffer)))
}
