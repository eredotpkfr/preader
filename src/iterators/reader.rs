use crate::{
    PReaderState,
    types::{config::PReaderIteratorConfig, core::AutoSaveStateFn},
};

pub trait PReaderIterator {
    fn config(&self) -> &PReaderIteratorConfig;
    fn state(&mut self) -> &mut PReaderState;

    fn percent(&mut self) -> f64 {
        self.state().percent()
    }

    fn saver(&self) -> AutoSaveStateFn {
        let enabled = self.config().auto_save_state;
        let noop: AutoSaveStateFn = |_, _| Ok(());
        let save: AutoSaveStateFn = |state, threshold| {
            let delta = state.position - state.manager.last_saved_position;

            if delta > 0 && delta >= threshold {
                state.save()?;
            }

            Ok(())
        };

        if enabled { save } else { noop }
    }
}
