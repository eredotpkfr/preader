use sha2::{Digest, Sha256};

/// Returns the lowercase hexadecimal SHA-256 digest of `data`
///
/// # Examples
///
/// ```
/// use preader::hash;
///
/// let digest = hash::sha256("foo");
/// let expected = "2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae";
///
/// assert_eq!(digest, expected);
/// ```
pub fn sha256(data: impl AsRef<[u8]>) -> String {
    hex::encode(Sha256::digest(data))
}
