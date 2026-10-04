use std::{
    fs::File,
    io::{BufReader, Seek, SeekFrom},
    path::Path,
};

use crate::{
    IteratorOptions, Progress, Result, enums::skip::Skip, types::core::FileReader,
    utils::file::starts_mid_item,
};

#[derive(Debug)]
pub struct Window {
    pub position: u64,
    pub skipping: Option<u64>,
    pub end: u64,
}

impl Window {
    pub fn open(&self, path: &Path, capacity: usize) -> Result<FileReader> {
        let mut file = File::open(path)?;

        file.seek(SeekFrom::Start(self.position))?;

        Ok(BufReader::with_capacity(capacity.max(1), file))
    }

    pub(crate) fn progress(
        &self,
        path: &Path,
        options: IteratorOptions,
        skip: Skip,
    ) -> Result<Progress> {
        Ok(Progress {
            end: self.end,
            limit: options.limit,
            yielded: 0,
            skipping: self.skip_items(path, skip.boundary())?,
        })
    }

    fn skip_items(&self, path: &Path, boundary: Option<u8>) -> Result<u64> {
        let Some(count) = self.skipping else {
            return Ok(0);
        };
        let Some(byte) = boundary else {
            return Ok(count);
        };

        let file = File::open(path)?;
        let misaligned = starts_mid_item(&file, self.position, byte)?;

        Ok(count.saturating_add(u64::from(misaligned)))
    }
}
