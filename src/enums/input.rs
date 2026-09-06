use std::path::Path;

use derive_more::From;

use crate::{
    Mismatch, Result, State, StateManager, manager::STATE_FILE_SUFFIX,
    types::config::reader::Config, utils::path::path_stem,
};

#[derive(Debug, Default, From)]
pub enum StateInput {
    #[default]
    Auto,
    #[from(&str, &String, String)]
    Name(String),
    #[from]
    Object(State),
}

impl<T: Into<StateInput>> From<Option<T>> for StateInput {
    fn from(state: Option<T>) -> Self {
        state.map(Into::into).unwrap_or_default()
    }
}

impl StateInput {
    pub(crate) fn resolve(self, config: &Config, file: &Path) -> Result<State> {
        let manager = StateManager::from(config);

        let name = match self {
            Self::Object(state) => {
                if !config.verify_state {
                    return Ok(state);
                }

                if state.file.path != file {
                    return Err(Mismatch::Path {
                        saved: state.file.path.clone(),
                        current: file.to_path_buf(),
                    }
                    .into());
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
