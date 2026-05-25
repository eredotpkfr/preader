use std::{
    fs::File,
    io::{Read, Result},
    path::Path,
};

use sha2::{Digest, Sha256};

const FINGERPRINT_BYTES: usize = 4096;

pub fn fingerprint(path: &Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; FINGERPRINT_BYTES];
    let read_count = file.read(&mut buffer)?;

    Ok(hex::encode(Sha256::digest(&buffer[..read_count])))
}
