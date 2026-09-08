use std::{
    fs::File,
    io::{BufReader, Seek, SeekFrom},
    path::Path,
};

use crate::{Progress, Result, types::core::Reader, utils::file::starts_mid_item};

#[derive(Debug)]
pub struct Window {
    pub position: u64,
    pub skipping: Option<u64>,
    pub end: u64,
}

impl Window {
    pub fn open(&self, path: &Path, capacity: usize) -> Result<Reader> {
        let mut file = File::open(path)?;

        file.seek(SeekFrom::Start(self.position))?;

        Ok(BufReader::with_capacity(capacity.max(1), file))
    }

    pub(crate) fn progress(
        &self,
        file: &File,
        boundary: Option<u8>,
        limit: u64,
    ) -> Result<Progress> {
        Ok(Progress::new(
            self.end,
            limit,
            self.skip_items(file, boundary)?,
        ))
    }

    fn skip_items(&self, file: &File, boundary: Option<u8>) -> Result<u64> {
        let Some(count) = self.skipping else {
            return Ok(0);
        };
        let misaligned = match boundary {
            Some(byte) => starts_mid_item(file, self.position, byte)?,
            None => false,
        };

        Ok(count.saturating_add(u64::from(misaligned)))
    }
}
