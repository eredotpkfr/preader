use std::{
    fs::File,
    io::{Error, Result},
};

use pyo3::prelude::*;

use crate::types::PReaderItem;

pub struct ProgressState {
    pub bytes_read: usize,
    pub total_bytes: usize,
}

impl TryFrom<&File> for ProgressState {
    type Error = Error;

    fn try_from(file: &File) -> Result<Self> {
        Ok(Self {
            bytes_read: 0,
            total_bytes: file.metadata()?.len() as usize,
        })
    }
}

impl ProgressState {
    pub fn yield_item<T>(&mut self, value: Py<T>, advance: usize) -> PReaderItem {
        self.bytes_read += advance;

        (self, value).into()
    }
}
