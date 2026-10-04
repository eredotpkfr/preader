use preader::{Config, PReader, StateManager, StateRegistry};

use crate::common::sandbox::Sandbox;

impl Sandbox {
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
}
