use crate::Config;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AutoSave {
    #[default]
    Off,
    Final,
    Every(u64),
}

impl From<&Config> for AutoSave {
    fn from(config: &Config) -> Self {
        match (config.auto_save_state, config.auto_save_state_bytes) {
            (false, _) => Self::Off,
            (true, 0) => Self::Final,
            (true, threshold) => Self::Every(threshold),
        }
    }
}

impl AutoSave {
    pub(crate) fn active(self) -> bool {
        !matches!(self, Self::Off)
    }

    pub(crate) fn floor(self, position: u64) -> u64 {
        match self {
            Self::Every(0) | Self::Off | Self::Final => position,
            Self::Every(threshold) => position - position % threshold,
        }
    }
}
