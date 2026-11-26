use std::{
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
    num::NonZeroU64,
    path::{Path, PathBuf},
};

use preader::{Config, PReader, State, StateRegistry};
use serde_json::Value;
use tempfile::TempDir;

#[cfg(unix)]
use crate::common::funcs::{canonical, state_data};
use crate::common::{
    constants::{TEST_FILE_NAME, TEST_LARGE_COPIES, TEST_LINE, TEST_STATE_NAME, TEST_TRACKED_NAME},
    funcs::drain,
};

#[cfg(unix)]
const RECORDED_SIZE: u64 = 64;

#[derive(Debug)]
pub struct Sandbox {
    config: Config,
    dir: TempDir,
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl Sandbox {
    pub fn new(config: Config) -> Self {
        let dir = TempDir::new().unwrap();
        let config = Config {
            state_dir: dir.path().join("states"),
            ..config
        };

        Self { config, dir }
    }

    pub fn path(&self) -> &Path {
        self.dir.path()
    }

    pub fn state_dir(&self) -> &Path {
        &self.config.state_dir
    }

    pub fn config(&self) -> Config {
        self.config.clone()
    }

    pub fn reader(&self) -> PReader {
        PReader::from(self.config())
    }

    pub fn reader_with(&self, config: Config) -> PReader {
        PReader::from(config)
    }

    pub fn reader_in(&self, name: &str, config: Config) -> PReader {
        PReader::from(Config {
            state_dir: self.path().join(name),
            ..config
        })
    }

    pub fn lenient(&self) -> PReader {
        self.reader_with(Config {
            verify_state: false,
            ..self.config()
        })
    }

    pub fn resuming(&self) -> PReader {
        self.reader_with(Config {
            auto_load_state: true,
            ..self.config()
        })
    }

    pub fn autosaving(&self, threshold: u64) -> PReader {
        self.reader_with(Config {
            auto_save_state: true,
            auto_save_state_bytes: NonZeroU64::new(threshold).unwrap(),
            ..self.config()
        })
    }

    pub fn capped(&self, capacity: usize) -> PReader {
        self.reader_with(Config {
            buffer_capacity: capacity,
            ..self.config()
        })
    }

    pub fn states(&self) -> StateRegistry {
        StateRegistry::from(&self.config)
    }

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

    pub fn state(&self, path: &Path) -> State {
        self.named_state(path, TEST_STATE_NAME)
    }

    pub fn named_state(&self, path: &Path, name: &str) -> State {
        self.reader().bytes(path).state(name).build().unwrap().state().clone()
    }

    #[cfg(unix)]
    pub fn state_at(&self, path: &Path, position: u64) -> State {
        let mut data = state_data(canonical(path));

        data.file.size = RECORDED_SIZE;
        data.position = position;
        data
    }

    pub fn saved(&self, path: &Path, name: &str) -> PathBuf {
        self.reader().bytes(path).state(name).build().unwrap().save().unwrap();
        self.states().path(name).unwrap()
    }

    pub fn save(&self, name: &str) -> PathBuf {
        let path = self.path().join(TEST_FILE_NAME);

        if !path.is_file() {
            self.line_file();
        }

        self.saved(&path, name)
    }

    pub fn checkpoint(&self, path: &Path, name: &str, items: u64) -> State {
        let mut bytes = self.reader().bytes(path).state(name).limit(items).build().unwrap();

        drain(&mut bytes);

        bytes.save().unwrap().clone()
    }

    pub fn recorded(&self, content: &[u8], read: u64) -> State {
        self.checkpoint(
            &self.write(TEST_TRACKED_NAME, content),
            TEST_STATE_NAME,
            read,
        )
    }

    pub fn persist(&self, state: &State) {
        let path = self.states().path(&state.name).unwrap();
        let sealed = State {
            checksum: state.checksum().unwrap(),
            ..state.clone()
        };

        path.parent().map(fs::create_dir_all).transpose().unwrap();

        fs::write(path, serde_json::to_string(&sealed).unwrap()).unwrap();
    }

    pub fn write_state(&self, name: &str, payload: impl AsRef<[u8]>) {
        let path = self.states().path(name).unwrap();

        path.parent().map(fs::create_dir_all).transpose().unwrap();

        fs::write(path, payload).unwrap();
    }

    pub fn tamper(&self, name: &str) {
        self.patch(name, |payload| payload["position"] = Value::from(999));
    }

    pub fn patch(&self, name: &str, change: impl FnOnce(&mut Value)) {
        let path = self.states().path(name).unwrap();
        let mut payload: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

        change(&mut payload);

        fs::write(&path, serde_json::to_string_pretty(&payload).unwrap()).unwrap();
    }

    pub fn payload(&self, state: &State) -> Value {
        let content = fs::read_to_string(self.states().path(&state.name).unwrap()).unwrap();
        let written: State = serde_json::from_str(&content).unwrap();

        assert_eq!(written, *state);

        serde_json::from_str(&content).unwrap()
    }
}
