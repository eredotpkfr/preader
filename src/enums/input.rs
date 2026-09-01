use std::path::Path;

use anyhow::anyhow;
use pyo3::{Borrowed, FromPyObject, PyAny, PyErr, exceptions::PyTypeError, prelude::*};

use crate::{
    Error, State, StateManager,
    manager::STATE_FILE_SUFFIX,
    types::{config::reader::Config, state::RESYNC_HINT},
    utils::path::path_stem,
};

pub(crate) enum StateInput {
    Object(State),
    Name(String),
    Auto,
}

impl<'a, 'py> FromPyObject<'a, 'py> for StateInput {
    type Error = PyErr;

    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if ob.is_none() {
            return Ok(Self::Auto);
        }

        if let Ok(state) = ob.extract::<State>() {
            return Ok(Self::Object(state));
        }

        if let Ok(name) = ob.extract::<String>() {
            return Ok(Self::Name(name));
        }

        Err(PyTypeError::new_err(
            "state must be None, a name (str), or a preader.State object",
        ))
    }
}

impl StateInput {
    pub(crate) fn resolve(self, config: &Config, file: &Path) -> Result<State, Error> {
        let manager = StateManager::from(config);

        let name = match self {
            Self::Object(state) => {
                if !config.verify_state {
                    return Ok(state);
                }

                if state.file.path != file {
                    return Err(Error::Message(anyhow!(
                        "file path mismatch (saved: '{}', current: '{}') {}",
                        state.file.path.display(),
                        file.display(),
                        RESYNC_HINT,
                    )));
                }

                state.verify()?;

                return Ok(state);
            }
            Self::Name(name) => path_stem(&name, STATE_FILE_SUFFIX),
            Self::Auto => manager.name(file),
        };

        if config.auto_load_state
            && let Ok(state) = manager.load(&name)
            && state.file.path == file
        {
            return Ok(state);
        }

        State::new(config, file, name)
    }
}
