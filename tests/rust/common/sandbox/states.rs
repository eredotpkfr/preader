use std::{fs, path::Path};

use preader::{IteratorBuild, IteratorRead, State};
use serde_json::Value;

use crate::common::{
    constants::{TEST_FILE_NAME, TEST_STATE_NAME, TEST_TRACKED_NAME},
    sandbox::Sandbox,
};

impl Sandbox {
    pub fn state(&self, path: &Path) -> State {
        self.named_state(path, TEST_STATE_NAME)
    }

    pub fn named_state(&self, path: &Path, name: &str) -> State {
        self.reader().bytes(path).state(name).build().unwrap().state().clone()
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
        let payload: Value = serde_json::from_str(&content).unwrap();

        assert_eq!(payload["name"], state.name.as_str());
        assert_eq!(payload["position"], state.position);
        assert_eq!(payload["_checksum"], state.checksum().unwrap());
        assert_eq!(payload["file"]["path"], state.file.path.to_str().unwrap());
        assert_eq!(payload["file"]["size"], state.file.size);
        assert_eq!(payload["file"]["mtime"], state.file.mtime.timestamp());
        assert_eq!(
            payload["file"]["fingerprint"],
            state.file.fingerprint.as_str()
        );
        assert_eq!(
            payload["timestamps"]["created_at"],
            state.timestamps.created_at.timestamp()
        );
        assert_eq!(
            payload["timestamps"]["updated_at"],
            state.timestamps.updated_at.timestamp()
        );

        payload
    }
}
