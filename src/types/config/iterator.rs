use crate::types::config::reader::Config;

#[derive(Clone)]
pub struct IteratorConfig {
    pub buffer_capacity: usize,
    pub auto_save_state: bool,
    pub auto_save_state_bytes: u64,
}

impl From<&Config> for IteratorConfig {
    fn from(config: &Config) -> Self {
        Self {
            buffer_capacity: config.buffer_capacity,
            auto_save_state: config.auto_save_state,
            auto_save_state_bytes: config.auto_save_state_bytes,
        }
    }
}
