use std::path::Path;

use derive_more::From;

use crate::{Error, Result, State, StateRegistry};

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
    pub(crate) fn resolve(self, registry: &StateRegistry, file: &Path) -> Result<State> {
        let (name, load) = match self {
            Self::Named(name) => (name, true),
            Self::Auto => (registry.autoname(file), registry.auto_load_state),
            Self::Existing(state) => {
                return registry.tracked(*state, file).map_err(Error::unsealed);
            }
        };

        registry.path(&name)?;

        if load {
            match registry.read(&name) {
                Err(Error::NotFound(_)) => {}
                read => return registry.tracked(read?, file),
            }
        }

        State::new(file, name)
    }
}
