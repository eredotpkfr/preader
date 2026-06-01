use std::path::PathBuf;

use pyo3::{Borrowed, FromPyObject, PyAny, PyErr, PyResult, exceptions::PyTypeError, prelude::*};

use crate::{
    State, StateError,
    types::{StateManager, config::Config},
};

pub(crate) enum StateInput {
    Object(State),
    Name(String),
    Default,
}

impl<'a, 'py> FromPyObject<'a, 'py> for StateInput {
    type Error = PyErr;

    fn extract(ob: Borrowed<'a, 'py, PyAny>) -> Result<Self, Self::Error> {
        if ob.is_none() {
            return Ok(Self::Default);
        }

        if let Ok(state) = ob.extract::<State>() {
            return Ok(Self::Object(state));
        }

        if let Ok(name) = ob.extract::<String>() {
            return Ok(Self::Name(name));
        }

        Err(PyTypeError::new_err("state must be None, a name (str), or a preader.State object"))
    }
}

impl StateInput {
    pub(crate) fn resolve(self, file: &PathBuf, config: &Config) -> PyResult<State> {
        let manager = StateManager::from(config);

        let name = match self {
            Self::Object(state) => {
                if config.verify_state {
                    state.verify()?;
                }
                return Ok(state);
            }
            Self::Name(name) => name,
            Self::Default => manager.name(file),
        };

        if config.auto_load_state {
            if let Ok(state) = manager.load(&name) {
                return Ok(state);
            }
        }

        State::new(config, file, name).map_err(StateError::from_anyhow)
    }
}
