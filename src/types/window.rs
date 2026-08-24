use std::{
    fs::File,
    io::{BufReader, Result, Seek, SeekFrom},
    path::Path,
};

pub struct Window {
    pub position: u64,
    pub end: u64,
    pub from_start: bool,
}

impl Window {
    pub fn open(&self, path: &Path, capacity: usize) -> Result<BufReader<File>> {
        let mut file = File::open(path)?;

        file.seek(SeekFrom::Start(self.position))?;

        Ok(BufReader::with_capacity(capacity.max(1), file))
    }
}
