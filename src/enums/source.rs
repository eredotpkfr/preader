use std::path::Path;

use derive_more::From;

use crate::{
    Error, Result, State, constants::STATE_FILE_EXTENSION, manager::StateManager,
    utils::path::path_stem,
};

#[derive(Debug, Default, From)]
pub enum StateSource {
    #[default]
    Auto,
    #[from(&str, &String, String)]
    Named(String),
    #[from(forward)]
    Existing(Box<State>),
}

impl<T: Into<Self>> From<Option<T>> for StateSource {
    fn from(state: Option<T>) -> Self {
        state.map(Into::into).unwrap_or_default()
    }
}

impl StateSource {
    pub(crate) fn resolve(self, manager: &StateManager, file: &Path) -> Result<State> {
        let name = match self {
            Self::Named(name) => path_stem(&name, STATE_FILE_EXTENSION),
            Self::Auto => manager.autoname(file),
            Self::Existing(state) => {
                let state = state.for_file(file)?;

                if manager.verify_state {
                    state.verify()?;
                }

                return Ok(state);
            }
        };

        manager.path(&name)?;

        if manager.auto_load_state {
            match manager.load(&name) {
                Ok(state) => return state.for_file(file),
                Err(Error::NotFound(_)) => {}
                Err(error) => return Err(error),
            }
        }

        State::new(manager.clone(), file, name)
    }
}
