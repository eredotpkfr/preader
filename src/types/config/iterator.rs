use crate::types::{config::reader::PReaderConfig, core::AutoSaveStateFn};

#[derive(Clone)]
pub struct PReaderIteratorConfig {
    pub buffer_capacity: usize,
    pub auto_save_state: bool,
    pub auto_save_state_bytes: u64,
    pub auto_load_state: bool,
}

impl From<&PReaderConfig> for PReaderIteratorConfig {
    fn from(config: &PReaderConfig) -> Self {
        Self {
            buffer_capacity: config.buffer_capacity,
            auto_save_state: config.auto_save_state,
            auto_save_state_bytes: config.auto_save_state_bytes,
            auto_load_state: config.auto_load_state,
        }
    }
}

impl PReaderIteratorConfig {
    pub fn saver(&self) -> AutoSaveStateFn {
        let noop: AutoSaveStateFn = |_, _| Ok(());
        let save: AutoSaveStateFn = |state, threshold| {
            let delta = state.position - state.manager.last_saved_position;

            if delta > 0 && delta >= threshold {
                state.save()?;
            }

            Ok(())
        };

        if self.auto_save_state { save } else { noop }
    }
}
