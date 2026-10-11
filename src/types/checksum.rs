use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    Result,
    types::{file::FileMetadata, state::StateData, time::Timestamps},
};

#[derive(Debug, Serialize)]
pub struct ChecksumBody<'a> {
    pub name: &'a str,
    pub file: &'a FileMetadata,
    pub position: u64,
    pub timestamps: &'a Timestamps,
}

impl<'a> From<&'a StateData> for ChecksumBody<'a> {
    fn from(data: &'a StateData) -> Self {
        Self {
            name: &data.name,
            file: &data.file,
            position: data.position,
            timestamps: &data.timestamps,
        }
    }
}

impl ChecksumBody<'_> {
    pub fn compute(&self) -> Result<String> {
        Ok(hex::encode(Sha256::digest(serde_json::to_string(self)?)))
    }
}
