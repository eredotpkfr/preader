use anyhow::Error;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    State,
    types::{file::FileMetadata, time::Timestamps},
};

#[derive(Serialize)]
pub struct ChecksumBody<'a> {
    pub name: &'a str,
    pub file: &'a FileMetadata,
    pub position: u64,
    pub timestamps: &'a Timestamps,
}

impl<'a> From<&'a State> for ChecksumBody<'a> {
    fn from(state: &'a State) -> Self {
        Self {
            name: &state.name,
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
