use std::num::NonZeroU64;

use crate::Config;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AutoSave {
    #[default]
    Never,
    EveryBytes(NonZeroU64),
}

impl From<&Config> for AutoSave {
    fn from(config: &Config) -> Self {
        if config.auto_save_state {
            Self::EveryBytes(config.auto_save_state_bytes)
        } else {
            Self::Never
        }
    }
}

impl AutoSave {
    pub(crate) fn active(self) -> bool {
        self != Self::Never
    }

    pub(crate) fn checkpoint(self, position: u64) -> u64 {
        match self {
            Self::Never => position,
            Self::EveryBytes(threshold) => position - position % threshold,
        }
    }
}
