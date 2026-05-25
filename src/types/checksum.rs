use anyhow::Error;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    PReaderState,
    types::{FileMetadata, Timestamps},
};

#[derive(Serialize)]
pub(crate) struct ChecksumBody<'a> {
    pub file: &'a FileMetadata,
    pub position: u64,
    pub timestamps: &'a Timestamps,
}

impl<'a> From<&'a PReaderState> for ChecksumBody<'a> {
    fn from(state: &'a PReaderState) -> Self {
        Self {
            file: &state.file,
            position: state.position,
            timestamps: &state.timestamps,
        }
    }
}

impl ChecksumBody<'_> {
    pub fn compute(&self) -> Result<String, Error> {
        Ok(hex::encode(Sha256::digest(serde_json::to_string(self)?)))
    }
}
