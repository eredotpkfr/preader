use std::{num::NonZeroU64, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    constants::{DEFAULT_AUTO_SAVE_STATE_BYTES, DEFAULT_BUFFER_CAPACITY, DEFAULT_VERIFY_STATE},
    utils::path,
};

/// Settings for buffering and for where and when states are saved, loaded and verified
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "preader", eq, frozen, from_py_object, get_all)
)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Config {
    /// Capacity of the read buffer in bytes, where `0` is treated as `1`
    pub buffer_capacity: usize,
    /// Directory states are saved in, or the current directory when empty
    pub state_dir: PathBuf,
    /// Whether iterators save every `auto_save_state_bytes`, at the end and when discarded
    pub auto_save_state: bool,
    /// Bytes of progress between two autosaves while `auto_save_state` is on
    pub auto_save_state_bytes: NonZeroU64,
    /// Whether an iterator created without a state resumes its auto-named state
    pub auto_load_state: bool,
    /// Whether a resumed or loaded state is checked for edits and for changes to its file
    pub verify_state: bool,
}

impl Default for Config {
    /// Creates the configuration whose states are saved in the user's cache directory
    ///
    /// The state directory is [`STATE_DIR_NAME`](crate::STATE_DIR_NAME) inside the user's cache
    /// directory, or a relative `preader` when there is none. Nothing is created until a
    /// state is saved. The other values are the `DEFAULT_*` constants, with autosave and auto-load
    /// off
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, STATE_DIR_NAME};
    ///
    /// let config = Config::default();
    ///
    /// assert!(config.state_dir.ends_with(STATE_DIR_NAME));
    /// ```
    fn default() -> Self {
        Self {
            buffer_capacity: DEFAULT_BUFFER_CAPACITY,
            state_dir: path::default_state_dir(),
            auto_save_state: false,
            auto_save_state_bytes: DEFAULT_AUTO_SAVE_STATE_BYTES,
            auto_load_state: false,
            verify_state: DEFAULT_VERIFY_STATE,
        }
    }
}
