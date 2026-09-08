use std::{fs, path::PathBuf};

use preader::{Config, PReader};
use tempfile::TempDir;

pub fn write(tmp_dir: &TempDir, name: &str, content: &[u8]) -> PathBuf {
    let path = tmp_dir.path().join(name);

    fs::write(&path, content).unwrap();

    path
}

pub fn reader(tmp_dir: &TempDir, config: Config) -> PReader {
    PReader::from(Config {
        state_dir: tmp_dir.path().join("states"),
        ..config
    })
}
