use std::{
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use crate::common::{
    constants::{TEST_FILE_NAME, TEST_LARGE_COPIES, TEST_LINE},
    sandbox::Sandbox,
};

impl Sandbox {
    pub fn write(&self, name: &str, content: &[u8]) -> PathBuf {
        let path = self.path().join(name);

        path.parent().map(fs::create_dir_all).transpose().unwrap();
        fs::write(&path, content).unwrap();

        path
    }

    pub fn file(&self, content: &[u8]) -> PathBuf {
        self.write(TEST_FILE_NAME, content)
    }

    pub fn line_file(&self) -> PathBuf {
        self.file(TEST_LINE)
    }

    pub fn empty_file(&self) -> PathBuf {
        self.write("empty.bin", b"")
    }

    pub fn large_file(&self) -> PathBuf {
        self.write("large.bin", &TEST_LINE.repeat(TEST_LARGE_COPIES))
    }

    pub fn block_states(&self) {
        fs::write(self.state_dir(), b"not a directory").unwrap();
    }

    pub fn dir_at(&self, name: &str) -> PathBuf {
        let path = self.path().join(name);

        fs::create_dir_all(&path).unwrap();

        path
    }

    pub fn append(&self, path: &Path, content: &[u8]) {
        OpenOptions::new().append(true).open(path).unwrap().write_all(content).unwrap();
    }

    pub fn overwrite(&self, path: &Path, offset: u64, content: &[u8]) {
        let mut file = OpenOptions::new().write(true).open(path).unwrap();

        file.seek(SeekFrom::Start(offset)).unwrap();
        file.write_all(content).unwrap();
    }

    pub fn truncate(&self, path: &Path, size: u64) {
        OpenOptions::new().write(true).open(path).unwrap().set_len(size).unwrap();
    }
}
