use std::num::NonZeroU64;

use crate::Config;

/// Policy for saving an iterator's state without an explicit [`save`](crate::PReaderIterator::save)
///
/// It is derived from [`Config::auto_save_state`] and [`Config::auto_save_state_bytes`]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AutoSave {
    /// Never saves on its own
    #[default]
    Never,
    /// Saves each time this many bytes are finished since the last autosave, at the end and on drop
    EveryBytes(NonZeroU64),
}

impl From<&Config> for AutoSave {
    /// Converts the autosave settings of `config` into a policy
    ///
    /// The result is [`AutoSave::EveryBytes`] with [`Config::auto_save_state_bytes`] when
    /// [`Config::auto_save_state`] is on, and [`AutoSave::Never`] otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{AutoSave, Config};
    ///
    /// let config = Config {
    ///     auto_save_state: true,
    ///     ..Config::default()
    /// };
    /// let autosave = AutoSave::from(&config);
    ///
    /// assert_eq!(autosave, AutoSave::EveryBytes(config.auto_save_state_bytes));
    /// ```
    fn from(config: &Config) -> Self {
        if config.auto_save_state {
            Self::EveryBytes(config.auto_save_state_bytes)
        } else {
            Self::Never
        }
    }
}

impl AutoSave {
    /// Returns whether the policy saves at all
    pub(crate) fn active(self) -> bool {
        self != Self::Never
    }

    /// Returns the autosave baseline for `position`, rounded down to a multiple of the threshold
    ///
    /// A resumed iterator therefore saves at the next multiple instead of a full threshold later
    pub(crate) fn checkpoint(self, position: u64) -> u64 {
        match self {
            Self::Never => position,
            Self::EveryBytes(threshold) => position - position % threshold,
        }
    }
}
