use std::{
    fs::{self, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use chrono::Timelike;
use preader::{
    Config, IteratorBuild, IteratorRead, PReader, State, StateData, StateManager, StateRegistry,
    Timestamps,
};
use serde_json::Value;
use tempfile::TempDir;

use crate::common::constants::{
    TEST_FILE_NAME, TEST_LARGE_COPIES, TEST_LINE, TEST_STATE_NAME, TEST_TRACKED_NAME,
};
#[cfg(unix)]
use crate::common::{constants::TEST_RECORDED_SIZE, funcs::state_data};

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
        PReader::from(Config {
            state_dir: self.state_dir().to_path_buf(),
            ..config
        })
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
            auto_save_state_bytes: threshold,
            ..self.config()
        })
    }

    pub fn capped(&self, capacity: usize) -> PReader {
        self.reader_with(Config {
            buffer_capacity: capacity,
            ..self.config()
        })
    }

    pub fn manager(&self) -> StateManager {
        StateManager::from(&self.config())
    }

    pub fn states(&self) -> StateRegistry {
        StateRegistry::from(&self.config())
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
        let mut data = state_data(path.to_path_buf());

        data.file.size = TEST_RECORDED_SIZE;
        data.position = position;

        self.manager().state(data)
    }

    pub fn saved(&self, path: &Path, name: &str) -> State {
        let mut state = self.named_state(path, name);

        state.save().unwrap();

        state
    }

    pub fn save(&self, name: &str) -> State {
        let path = self.path().join(TEST_FILE_NAME);

        if !path.is_file() {
            self.line_file();
        }

        self.saved(&path, name)
    }

    pub fn checkpoint(&self, path: &Path, name: &str, items: u64) -> State {
        let mut bytes = self.reader().bytes(path).state(name).limit(items).build().unwrap();

        while bytes.read().unwrap().is_some() {}

        bytes.state().save().unwrap();
        bytes.state().clone()
    }

    pub fn stored(&self, path: &Path, name: &str) -> State {
        let reader = self.lenient();
        let mut state = reader.bytes(path).state(name).build().unwrap().state().clone();

        state.save().unwrap();

        reader.states().load(name).unwrap()
    }

    pub fn recorded(&self, content: &[u8], read: u64) -> State {
        let path = self.write(TEST_TRACKED_NAME, content);
        let mut bytes =
            self.reader().bytes(&path).state(TEST_STATE_NAME).limit(read).build().unwrap();

        while bytes.read().unwrap().is_some() {}

        bytes.state().clone()
    }

    pub fn payload(&self, state: &State) -> Value {
        let content = fs::read_to_string(state.path().unwrap()).unwrap();
        let written: StateData = serde_json::from_str(&content).unwrap();
        let mut expected = (**state).clone();

        expected.timestamps = Timestamps {
            created_at: expected.timestamps.created_at.with_nanosecond(0).unwrap(),
            updated_at: expected.timestamps.updated_at.with_nanosecond(0).unwrap(),
        };

        assert_eq!(written, expected);

        serde_json::from_str(&content).unwrap()
    }
}
