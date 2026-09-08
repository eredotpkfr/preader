use std::{
    fs::File,
    io::{Read, Result, Seek, SeekFrom},
    path::Path,
};

use sha2::{Digest, Sha256};

pub fn fingerprint(path: &Path, window: u64) -> Result<String> {
    let mut buffer = Vec::with_capacity(window as usize);

    File::open(path)?.take(window).read_to_end(&mut buffer)?;

    Ok(hex::encode(Sha256::digest(&buffer)))
}

pub fn starts_mid_item(file: &File, position: u64, boundary: u8) -> Result<bool> {
    let Some(previous) = position.checked_sub(1) else {
        return Ok(false);
    };

    let mut handle = file;
    let mut byte = [0u8; 1];

    handle.seek(SeekFrom::Start(previous))?;

    let read_count = handle.read(&mut byte)?;

    handle.seek(SeekFrom::Start(position))?;

    Ok(read_count == 1 && byte[0] != boundary)
}
