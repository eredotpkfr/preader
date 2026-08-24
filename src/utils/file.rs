use std::{
    fs::File,
    io::{Read, Result},
    os::unix::fs::FileExt,
    path::Path,
};

use sha2::{Digest, Sha256};

const FINGERPRINT_BYTES: u64 = 4096;

pub fn fingerprint(path: &Path) -> Result<String> {
    let mut buffer = Vec::with_capacity(FINGERPRINT_BYTES as usize);

    File::open(path)?.take(FINGERPRINT_BYTES).read_to_end(&mut buffer)?;

    Ok(hex::encode(Sha256::digest(&buffer)))
}

pub fn starts_mid_item(file: &File, position: u64, boundary: u8) -> Result<bool> {
    let Some(previous) = position.checked_sub(1) else {
        return Ok(false);
    };

    let mut byte = [0u8; 1];

    Ok(file.read_at(&mut byte, previous)? == 1 && byte[0] != boundary)
}
