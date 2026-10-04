use crate::Config;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AutoSave {
    #[default]
    Never,
    AtEnd,
    EveryBytes(u64),
}

impl From<&Config> for AutoSave {
    fn from(config: &Config) -> Self {
        match (config.auto_save_state, config.auto_save_state_bytes) {
            (false, _) => Self::Never,
            (true, 0) => Self::AtEnd,
            (true, threshold) => Self::EveryBytes(threshold),
        }
    }
}

impl AutoSave {
    pub(crate) fn active(self) -> bool {
        !matches!(self, Self::Never)
    }

    pub(crate) fn floor(self, position: u64) -> u64 {
        match self {
            Self::EveryBytes(0) | Self::Never | Self::AtEnd => position,
            Self::EveryBytes(threshold) => position - position % threshold,
        }
    }
}
