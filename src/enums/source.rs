use std::path::Path;

use derive_more::From;

use crate::{Error, Result, State, StateRegistry};

/// Where a builder takes the state of a new iterator from
///
/// [`PReaderIteratorBuilder::state`](crate::PReaderIteratorBuilder::state) accepts anything
/// that converts into it: a name as `&str`, `&String` or `String` gives [`Named`], a [`State`]
/// or `Box<State>` gives [`Existing`], and `None` gives [`Auto`]. A missing saved state starts a
/// fresh one, and so does [`Auto`] with [`Config::auto_load_state`] off. Any other problem with
/// a saved state is an error
///
/// # Examples
///
/// ```
/// use preader::StateSource;
///
/// let source = StateSource::from("job-1");
///
/// assert!(matches!(source, StateSource::Named(name) if name == "job-1"));
/// ```
///
/// [`Config::auto_load_state`]: crate::Config::auto_load_state
/// [`Auto`]: StateSource::Auto
/// [`Named`]: StateSource::Named
/// [`Existing`]: StateSource::Existing
#[derive(Debug, Default, From)]
pub enum StateSource {
    /// Resumes the auto-named state if [`auto_load_state`](crate::Config::auto_load_state) is on
    ///
    /// Otherwise it starts a fresh state, whose save replaces the saved auto-named state
    #[default]
    Auto,
    /// Resumes the state saved under this name, or starts a fresh one under it
    #[from(&str, &String, String)]
    Named(String),
    /// Resumes from this passed [`State`], ignoring what is saved in the state directory
    ///
    /// Unless it equals the saved state, its first save overwrites the saved state, even one with
    /// more progress
    #[from(forward)]
    Existing(Box<State>),
}

impl<T: Into<Self>> From<Option<T>> for StateSource {
    /// Converts `Some` like its content and `None` into [`StateSource::Auto`]
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::StateSource;
    ///
    /// let source = StateSource::from(None::<&str>);
    ///
    /// assert!(matches!(source, StateSource::Auto));
    /// ```
    fn from(state: Option<T>) -> Self {
        state.map(Into::into).unwrap_or_default()
    }
}

impl StateSource {
    /// Returns the state a new iterator over the canonical `file` starts from
    ///
    /// [`Named`](Self::Named) loads the saved state, [`Auto`](Self::Auto) does so only with
    /// [`auto_load_state`](crate::Config::auto_load_state) on, and both create a fresh state at 0
    /// when nothing is loaded. [`Existing`](Self::Existing) is checked like a loaded state without
    /// reading the state directory
    ///
    /// # Errors
    ///
    /// [`Named`](Self::Named) returns [`Error::Name`] when the name is not a valid state name, and
    /// both [`Named`](Self::Named) and [`Auto`](Self::Auto) return [`Error::Alias`] when a part of
    /// the name is a symlink or a case or Unicode alias. A loaded state returns the errors of
    /// [`StateRegistry::read`] except [`Error::NotFound`], and a loaded or passed state the errors
    /// of [`StateRegistry::tracked`], with [`Mismatch::Checksum`](crate::Mismatch::Checksum)
    /// reported as [`Mismatch::Unsealed`](crate::Mismatch::Unsealed) for
    /// [`Existing`](Self::Existing). A fresh state fails with [`Error::Io`] when `file` cannot be
    /// read, [`Error::InvalidMtime`] when its mtime is out of range and [`Error::Serde`] when its
    /// path is not valid UTF-8
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
