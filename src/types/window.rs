use std::{
    fs::File,
    io::{BufReader, Seek, SeekFrom},
    path::Path,
};

use crate::{Progress, Result, types::core::FileReader, utils::file};

#[derive(Debug)]
pub struct Window {
    pub position: u64,
    pub end: u64,
    pub limit: u64,
    pub skipping: Option<u64>,
    pub boundary: Option<u8>,
}

impl Window {
    pub fn open(&self, path: &Path, capacity: usize) -> Result<FileReader> {
        let mut file = File::open(path)?;

        file.seek(SeekFrom::Start(self.position))?;

        Ok(BufReader::with_capacity(capacity.max(1), file))
    }

    pub(crate) fn progress(&self, path: &Path) -> Result<Progress> {
        Ok(Progress {
            end: self.end,
            limit: self.limit,
            yielded: 0,
            skipping: self.skip_items(path)?,
            pending: 0,
        })
    }

    fn skip_items(&self, path: &Path) -> Result<u64> {
        let Some(count) = self.skipping else {
            return Ok(0);
        };
        let Some(byte) = self.boundary else {
            return Ok(count);
        };

        let file = File::open(path)?;
        let misaligned = file::starts_mid_item(&file, self.position, byte)?;

        Ok(count.saturating_add(u64::from(misaligned)))
    }
}
