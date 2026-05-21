use std::{
    fs::File,
    io::{Error, Result},
};

use pyo3::prelude::*;

use crate::types::PReaderItem;

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
    pub fn yield_item<T>(&mut self, value: Py<T>, consumed: usize) -> PReaderItem {
        self.bytes_read += consumed as u64;

        (self, value).into()
    }
}
