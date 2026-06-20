use pyo3::prelude::*;

use crate::{State, types::config::IteratorConfig};

#[pyclass(subclass)]
pub struct IteratorBase {
    pub config: IteratorConfig,
    pub state: State,
    pub end: u64,
    pub limit: u64,
    pub yielded: u64,
}

impl IteratorBase {
    pub(crate) fn new(config: IteratorConfig, mut state: State, end: u64, limit: u64) -> Self {
        let threshold = config.auto_save_state_bytes;

        state.manager.last_saved_position = if threshold > 0 {
            (state.position / threshold) * threshold
        } else {
            state.position
        };

        Self {
            config,
            state,
            end,
            limit,
            yielded: 0,
        }
    }

    #[inline]
    pub(crate) fn should_stop(&self) -> bool {
        self.state.position >= self.end || self.yielded >= self.limit
    }

    #[inline]
    pub(crate) fn count_yield(&mut self) {
        self.yielded += 1;
    }

    #[inline]
    pub(crate) fn advance(&mut self, bytes: u64) -> PyResult<()> {
        self.state.advance(bytes);

        if self.config.auto_save_state_bytes == 0 {
            return Ok(());
        }

        self.autosave(self.config.auto_save_state_bytes)
    }

    #[inline]
    pub(crate) fn finalize(&mut self) -> PyResult<()> {
        self.autosave(0)
    }

    #[inline]
    fn autosave(&mut self, threshold: u64) -> PyResult<()> {
        if !self.config.auto_save_state {
            return Ok(());
        }

        let delta = self.state.position - self.state.manager.last_saved_position;

        if delta > 0 && delta >= threshold {
            self.state.save()?;
        }

        Ok(())
    }
}

#[pymethods]
impl IteratorBase {
    #[getter]
    fn state(&self) -> State {
        self.state.clone()
    }

    fn percent(&self) -> f64 {
        self.state.percent()
    }
}

impl Drop for IteratorBase {
    fn drop(&mut self) {
        Python::try_attach(|_| {
            if let Err(err) = self.finalize() {
                eprintln!("preader: save failed: {err}");
            }
        });
    }
}
