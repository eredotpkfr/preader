use std::path::Path;

use preader::Config;
use tempfile::TempDir;

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
}
