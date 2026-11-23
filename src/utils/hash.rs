use sha2::{Digest, Sha256};

pub fn sha256(data: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(data))
}
