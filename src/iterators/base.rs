use pyo3::prelude::*;

use crate::{PReaderState, types::config::PReaderIteratorConfig};

#[pyclass(subclass)]
pub struct PReaderIteratorBase {
    pub config: PReaderIteratorConfig,
    pub state: PReaderState,
}

impl PReaderIteratorBase {
    pub(crate) fn new(config: PReaderIteratorConfig, mut state: PReaderState) -> Self {
        let threshold = config.auto_save_state_bytes;

        state.manager.last_saved_position = if threshold > 0 {
            (state.position / threshold) * threshold
        } else {
            state.position
        };

        Self { config, state }
    }

    pub(crate) fn advance(&mut self, bytes: u64) -> PyResult<()> {
        self.state.advance(bytes);

        (self.config.saver())(&mut self.state, self.config.auto_save_state_bytes)
    }

    pub(crate) fn finalize(&mut self) -> PyResult<()> {
        (self.config.saver())(&mut self.state, 0)
    }
}

#[pymethods]
impl PReaderIteratorBase {
    fn state(&self) -> PReaderState {
        self.state.clone()
    }

    fn percent(&self) -> f64 {
        self.state.percent()
    }
}

impl Drop for PReaderIteratorBase {
    fn drop(&mut self) {
        Python::try_attach(|_| {
            if let Err(e) = (self.config.saver())(&mut self.state, 0) {
                eprintln!("preader: save failed: {e}");
            }
        });
    }
}
