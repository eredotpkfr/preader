use crate::types::config::reader::PReaderConfig;

#[derive(Clone)]
pub struct PReaderIteratorConfig {
    pub buffer_capacity: usize,
    pub auto_save_state: bool,
    pub auto_load_state: bool,
}

impl From<&PReaderConfig> for PReaderIteratorConfig {
    fn from(config: &PReaderConfig) -> Self {
        Self {
            buffer_capacity: config.buffer_capacity,
            auto_save_state: config.auto_save_state,
            auto_load_state: config.auto_load_state,
        }
    }
}
