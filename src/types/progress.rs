use std::{
    fs::File,
    io::{Error, Result},
};

pub struct ProgressState {
    pub bytes_read: u64,
    pub total_bytes: u64,
}

impl TryFrom<&File> for ProgressState {
    type Error = Error;

    fn try_from(file: &File) -> Result<Self> {
        Ok(Self {
            bytes_read: 0,
            total_bytes: file.metadata()?.len(),
        })
    }
}

impl ProgressState {
    pub fn advance(&mut self, consumed: usize) {
        self.bytes_read += consumed as u64;
    }
}
