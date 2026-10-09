use std::path::Path;

use derive_more::From;

use crate::{Error, Result, State, manager::StateManager};

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
            Self::Named(name) => name,
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
                Err(Error::NotFound(_)) => {}
                loaded => return loaded?.for_file(file),
            }
        }

        State::new(manager.clone(), file, name)
    }
}
