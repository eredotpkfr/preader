use std::path::Path;

use pyo3::{Borrowed, FromPyObject, PyAny, PyErr, PyResult, exceptions::PyTypeError, prelude::*};

use crate::{State, StateError, StateManager, types::config::Config};

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
    pub(crate) fn resolve(self, config: &Config, file: &Path) -> PyResult<State> {
        let manager = StateManager::from(config);

        let name = match self {
            Self::Object(state) => {
                if config.verify_state {
                    state.verify()?;
                }
                return Ok(state);
            }
            Self::Name(name) => name,
            Self::Auto => manager.name(file),
        };

        if config.auto_load_state {
            if let Ok(state) = manager.load(&name) {
                return Ok(state);
            }
        }

        State::new(config, file, name).map_err(StateError::from_anyhow)
    }
}
