use std::{fs, path::PathBuf};

use tempfile::TempDir;

pub fn write(tmp_dir: &TempDir, name: &str, content: &[u8]) -> PathBuf {
    let path = tmp_dir.path().join(name);

    fs::write(&path, content).unwrap();

    path
}
