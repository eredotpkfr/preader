use std::{
    fs::File,
    io::{Read, Result},
    path::Path,
};

use sha2::{Digest, Sha256};

const FINGERPRINT_BYTES: u64 = 4096;

pub fn fingerprint(path: &Path) -> Result<String> {
    let mut buffer = Vec::with_capacity(FINGERPRINT_BYTES as usize);

    File::open(path)?.take(FINGERPRINT_BYTES).read_to_end(&mut buffer)?;

    Ok(hex::encode(Sha256::digest(&buffer)))
}
