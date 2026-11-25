use std::{
    fs,
    path::{Path, PathBuf},
};

use serde_json::to_string_pretty;
use time::UtcDateTime;

use crate::{
    Error, Mismatch, Result, State, StateIterator,
    constants::{CURRENT_DIR, STATE_FILE_EXTENSION, TMP_FILE_EXTENSION},
    macros::ensure,
    types::config::Config,
    utils::{file, hash, path},
};

/// Registry of the states saved in one state directory
///
/// Every reader exposes one as `states`. Each state is a JSON file named `<name>.state.json`
/// under the state directory, and a `/` in a name becomes a subdirectory
#[cfg_attr(feature = "python", pyo3::pyclass(module = "preader"))]
#[derive(Clone, Debug)]
pub struct StateRegistry {
    /// Directory the state files are saved in
    pub(crate) state_dir: PathBuf,
    /// Whether loaded states are checked against their file, from [`Config::verify_state`]
    pub(crate) verify_state: bool,
    /// Whether an auto-named state is resumed, from [`Config::auto_load_state`]
    pub(crate) auto_load_state: bool,
}

impl Default for StateRegistry {
    /// Creates a registry for [`Config::default`], which saves in the user's cache directory
    fn default() -> Self {
        Self::from(&Config::default())
    }
}

impl From<&Config> for StateRegistry {
    /// Creates a registry from [`state_dir`], [`verify_state`] and [`auto_load_state`] of `config`
    ///
    /// An empty [`state_dir`] means the current directory
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::{Path, PathBuf};
    ///
    /// use preader::{Config, StateRegistry};
    ///
    /// let registry = StateRegistry::from(&Config {
    ///     state_dir: PathBuf::new(),
    ///     ..Config::default()
    /// });
    ///
    /// assert_eq!(registry.state_dir(), Path::new("."));
    /// ```
    ///
    /// [`state_dir`]: Config::state_dir
    /// [`verify_state`]: Config::verify_state
    /// [`auto_load_state`]: Config::auto_load_state
    fn from(config: &Config) -> Self {
        Self {
            state_dir: if config.state_dir.is_empty() {
                CURRENT_DIR.into()
            } else {
                config.state_dir.clone()
            },
            verify_state: config.verify_state,
            auto_load_state: config.auto_load_state,
        }
    }
}

impl StateRegistry {
    /// Returns the directory the state files are saved in
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// assert_eq!(reader.states().state_dir(), dir.path().join("states"));
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }

    /// Returns an iterator over the names of every saved state
    ///
    /// The state directory is walked recursively, following only a symlinked state directory
    /// itself, and names use `/` as separator on every platform. The order is unspecified.
    /// Entries that do not map back to a valid state name (temporary files, symlinks, invalid
    /// names, non-UTF-8 paths) are skipped silently, and a missing state directory yields no
    /// names
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the state directory exists but is not a directory. Each item is
    /// an [`Error::Io`] when walking part of the directory fails
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// reader.lines(&file).state("logs/today").build()?.save()?;
    ///
    /// let names = reader.states().names()?.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(names, ["logs/today"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn names(&self) -> Result<StateIterator> {
        StateIterator::new(self, None)
    }

    /// Returns an iterator over the names of the saved states that match `pattern`
    ///
    /// `pattern` is a regular expression in the syntax of the `regex` crate and matches anywhere
    /// in the name, so anchor it with `^` and `$` to match a whole name. Otherwise it behaves
    /// like [`names`](StateRegistry::names)
    ///
    /// # Errors
    ///
    /// Returns [`Error::Regex`] when `pattern` is not a valid regular expression. Returns
    /// [`Error::Io`] when the state directory exists but is not a directory. Each item is an
    /// [`Error::Io`] when walking part of the directory fails
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// reader.lines(&file).state("logs/a").build()?.save()?;
    /// reader.lines(&file).state("jobs/b").build()?.save()?;
    ///
    /// let names = reader.states().search("^logs/")?.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(names, ["logs/a"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn search(&self, pattern: &str) -> Result<StateIterator> {
        StateIterator::new(self, Some(pattern))
    }

    /// Loads the state saved under `name`
    ///
    /// With [`Config::verify_state`] on, the state is also checked against its tracked file, so a
    /// file that changed since the state was created or last resynced is rejected
    ///
    /// # Errors
    ///
    /// Returns [`Error::Name`] when `name` is not a valid state name, [`Error::Alias`] when a part
    /// of it is a symlink or a case or Unicode alias, [`Error::NotFound`] when nothing is saved
    /// under it, [`Error::Corrupt`] when the state file does not parse, [`Error::Mismatch`] with
    /// [`Mismatch::Name`] when it belongs to another name, and [`Error::Io`] when it cannot be
    /// read. With [`Config::verify_state`] on, also returns [`Error::Mismatch`] with
    /// [`Mismatch::Checksum`] when the state file was edited, with [`Mismatch::Missing`],
    /// [`Mismatch::Identity`], [`Mismatch::Size`] or [`Mismatch::Mtime`] when the tracked file
    /// changed, [`Error::Io`] when that file cannot be read, and [`Error::InvalidMtime`] when its
    /// mtime is out of range
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\nbar\n")?;
    ///
    /// reader.lines(&file).state("job-1").start(4).build()?.save()?;
    ///
    /// assert_eq!(reader.states().load("job-1")?.position, 4);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn load(&self, name: &str) -> Result<State> {
        self.verified(self.read(name)?)
    }

    /// Loads the state saved under `name`, or returns `None` when loading fails
    ///
    /// Every error of [`load`](StateRegistry::load) becomes `None`, including a corrupt state or
    /// a failed verification, so use [`load`](StateRegistry::load) to tell a missing state from
    /// a broken one
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// reader.lines(&file).state("job-1").build()?.save()?;
    ///
    /// assert!(reader.states().find("job-1").is_some());
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn find(&self, name: &str) -> Option<State> {
        self.load(name).ok()
    }

    /// Loads every saved state
    ///
    /// The states are loaded in the order of [`names`](StateRegistry::names) and the call is
    /// all or nothing: the first state that fails to load fails the whole call
    ///
    /// # Errors
    ///
    /// Returns the errors of [`names`](StateRegistry::names) and of its items, and the errors of
    /// [`load`](StateRegistry::load) for the first state that cannot be loaded
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// reader.lines(&file).state("job-1").build()?.save()?;
    /// reader.lines(&file).state("job-2").build()?.save()?;
    ///
    /// assert_eq!(reader.states().all()?.len(), 2);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn all(&self) -> Result<Vec<State>> {
        self.names()?.map(|name| self.load(&name?)).collect()
    }

    /// Returns whether a state file is saved under `name`
    ///
    /// Only the presence of the file is checked, without parsing or verifying it, and an
    /// invalid or aliased name gives `false`
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// reader.lines(&file).state("job-1").build()?.save()?;
    ///
    /// assert!(reader.states().exists("job-1"));
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn exists(&self, name: &str) -> bool {
        self.path(name).is_ok_and(|path| path.is_file())
    }

    /// Returns the number of saved states
    ///
    /// Counts the names [`names`](StateRegistry::names) yields without loading them, so corrupt
    /// states are counted too
    ///
    /// # Errors
    ///
    /// Returns the errors of [`names`](StateRegistry::names) and of its items
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// reader.lines(&file).state("job-1").build()?.save()?;
    ///
    /// assert_eq!(reader.states().count()?, 1);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn count(&self) -> Result<usize> {
        self.names()?.map(|name| name.map(|_| 1)).sum()
    }

    /// Returns the state name used for `file` when an iterator is built without a state
    ///
    /// The name is the SHA-256 hex digest of the path bytes exactly as given, so another spelling
    /// of the same path gives another name. Iterators hash the canonical path their state records
    /// in [`FileMetadata::path`](crate::FileMetadata::path), which on Windows lacks the `\\?\`
    /// prefix that [`std::fs::canonicalize`] adds
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// let lines = reader.lines(&file).build()?;
    /// let state = lines.state();
    ///
    /// assert_eq!(state.name, reader.states().autoname(&state.file.path));
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn autoname(&self, file: &Path) -> String {
        hash::sha256(file.as_os_str().as_encoded_bytes())
    }

    /// Returns the path of the state file for `name`, whether or not it exists
    ///
    /// Each `/` in `name` becomes a subdirectory of the state directory and the file name ends
    /// with the [`STATE_FILE_EXTENSION`] extension
    ///
    /// # Errors
    ///
    /// Returns [`Error::Name`] when `name` is not a valid state name and [`Error::Alias`] when a
    /// part of it is a symlink or a case or Unicode alias
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// let path = reader.states().path("job-1")?;
    ///
    /// assert!(path.ends_with("job-1.state.json"));
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn path(&self, name: &str) -> Result<PathBuf> {
        self.locate(name, &[STATE_FILE_EXTENSION])
    }

    /// Points the state saved under `name` at the file `path` and saves it
    ///
    /// Use it after the tracked file moved, grew or changed. The canonical `path` and its size,
    /// mtime and fingerprint are recorded, while the position (clamped to the new size) and
    /// [`created_at`](crate::Timestamps::created_at) are kept. With [`Config::verify_state`] on,
    /// only the checksum and the fingerprint, which covers at most the first
    /// [`FINGERPRINT_SAMPLE_BYTES`](crate::FINGERPRINT_SAMPLE_BYTES), are checked, so this accepts
    /// the changes that fail with [`Mismatch::Path`], [`Mismatch::Missing`], [`Mismatch::Size`] or
    /// [`Mismatch::Mtime`]. With verification off, any readable regular file is accepted
    ///
    /// The name is kept, so after a move an auto-named state resumes only when its name is passed
    /// as the state. A build without one looks up the name of the new path. An iterator
    /// still holding the old state is a stale writer now, so its next save, manual or autosave,
    /// fails with [`Mismatch::Stale`]
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when `path` cannot be canonicalized or is not a readable regular
    /// file, and [`Error::InvalidMtime`] when its mtime is out of range. Returns the errors of
    /// [`load`](StateRegistry::load) for reading the saved state, without its file checks. With
    /// [`Config::verify_state`] on, returns [`Error::Mismatch`] with [`Mismatch::Checksum`] when
    /// the state file was edited and [`Mismatch::Identity`] when the first bytes of `path` do not
    /// match the recorded fingerprint. Returns [`Error::Serde`] when `path` is not valid UTF-8 and
    /// [`Error::Io`] when the state cannot be written
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\nbar\n")?;
    ///
    /// reader.lines(&file).state("job-1").start(4).build()?.save()?;
    ///
    /// std::fs::write(&file, "foo\nbar\nbaz\n")?;
    ///
    /// reader.states().resync("job-1", &file)?;
    ///
    /// let resumed = reader.lines(&file).state("job-1").build()?;
    /// let read = resumed.collect::<preader::Result<Vec<_>>>()?;
    ///
    /// assert_eq!(read, ["bar", "baz"]);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn resync(&self, name: &str, path: impl AsRef<Path>) -> Result<State> {
        let path = dunce::canonicalize(path)?;
        let resynced = self.read(name)?.resynced(&path, self.verify_state)?;

        self.commit(&resynced)?;

        Ok(resynced)
    }

    /// Deletes the state file saved under `name`
    ///
    /// The file is removed without being parsed, so a corrupt state can be deleted too. An iterator
    /// that still holds the state writes it again on its next save, manual or autosave
    ///
    /// # Errors
    ///
    /// Returns [`Error::Name`] when `name` is not a valid state name, [`Error::Alias`] when a part
    /// of it is a symlink or a case or Unicode alias, [`Error::NotFound`] when no state file is
    /// saved under it, and [`Error::Io`] when the file cannot be removed, for example because a
    /// directory is in its place
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// reader.lines(&file).state("job-1").build()?.save()?;
    /// reader.states().delete("job-1")?;
    ///
    /// assert!(!reader.states().exists("job-1"));
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn delete(&self, name: &str) -> Result<()> {
        fs::remove_file(self.path(name)?).map_err(Error::not_found(name))
    }

    /// Deletes every saved state
    ///
    /// States are deleted one at a time in the order of [`names`](StateRegistry::names) without
    /// being parsed, so corrupt states are removed too. The first failure stops the call and
    /// leaves the remaining states in place, and emptied subdirectories are kept. An iterator that
    /// still holds a state writes it again on its next save, manual or autosave
    ///
    /// # Errors
    ///
    /// Returns the errors of [`names`](StateRegistry::names) and of its items, and the errors of
    /// [`delete`](StateRegistry::delete) for the first state that cannot be deleted
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    /// let file = dir.path().join("data.txt");
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// std::fs::write(&file, "foo\n")?;
    ///
    /// reader.lines(&file).state("job-1").build()?.save()?;
    /// reader.lines(&file).state("job-2").build()?.save()?;
    /// reader.states().clear()?;
    ///
    /// assert_eq!(reader.states().count()?, 0);
    /// # Ok::<(), preader::Error>(())
    /// ```
    pub fn clear(&self) -> Result<()> {
        self.names()?.try_for_each(|name| self.delete(&name?))
    }

    /// Returns a temporary path like the one a save of `name` writes to before renaming it
    ///
    /// The path is the state file path with a nanosecond timestamp before the
    /// [`STATE_FILE_EXTENSION`] extension and the `tmp` extension after it. This is a testing hook
    ///
    /// # Errors
    ///
    /// Returns [`Error::Name`] when `name` is not a valid state name and [`Error::Alias`] when a
    /// part of it is a symlink or a case or Unicode alias
    ///
    /// # Examples
    ///
    /// ```
    /// use preader::{Config, PReader};
    ///
    /// let dir = tempfile::tempdir()?;
    ///
    /// let reader = PReader::from(Config {
    ///     state_dir: dir.path().join("states"),
    ///     ..Config::default()
    /// });
    ///
    /// let path = reader.states().tmp_path("job-1")?;
    ///
    /// assert_eq!(path.extension(), Some("tmp".as_ref()));
    /// # Ok::<(), preader::Error>(())
    /// ```
    #[cfg(feature = "testing")]
    pub fn tmp_path(&self, name: &str) -> Result<PathBuf> {
        self.tmp(name)
    }

    /// Writes `state` to its state file atomically, as pretty-printed JSON
    ///
    /// The JSON is written to a fresh [`tmp`](StateRegistry::tmp) file, synced and renamed over
    /// the state file, so a reader never sees a half-written state
    ///
    /// # Errors
    ///
    /// Returns [`Error::Name`] when the state's name is not a valid state name, [`Error::Alias`]
    /// when a part of it is a symlink or a case or Unicode alias, [`Error::Serde`] when the
    /// state cannot be serialized, and [`Error::Io`] when creating the directory or creating,
    /// writing, syncing or renaming the file fails
    pub(crate) fn commit(&self, state: &State) -> Result<()> {
        let path = self.path(&state.name)?;
        let tmp = self.tmp(&state.name)?;

        Ok(file::write_atomic(&path, &tmp, to_string_pretty(state)?)?)
    }

    /// Reads and parses the state saved under `name` without verifying it against its file
    ///
    /// # Errors
    ///
    /// Returns [`Error::Name`] when `name` is not a valid state name, [`Error::Alias`] when a part
    /// of it is a symlink or a case or Unicode alias, [`Error::NotFound`] when no regular file is
    /// saved there, [`Error::Io`] when it cannot be read, [`Error::Corrupt`] when it does not
    /// parse, and [`Error::Mismatch`] with [`Mismatch::Name`] when it belongs to another name
    pub(crate) fn read(&self, name: &str) -> Result<State> {
        let path = self.path(name)?;

        ensure!(path.is_file(), Error::NotFound(name.to_owned()));

        let content = fs::read(&path).map_err(Error::not_found(name))?;
        let state: State = serde_json::from_slice(&content).map_err(Error::corrupt(name))?;

        ensure!(
            state.name == name,
            Error::mismatch(name, Mismatch::Name { saved: state.name })
        );

        Ok(state)
    }

    /// Checks that `state` tracks `file` and then verifies it like [`verified`](Self::verified)
    ///
    /// # Errors
    ///
    /// Returns the errors of [`State::tracking`], including [`Mismatch::Path`] when `state` tracks
    /// another file, and of [`State::verify`] when [`verify_state`](Self::verify_state) is on
    pub(crate) fn tracked(&self, state: State, file: &Path) -> Result<State> {
        self.verified(state.tracking(file, self.verify_state)?)
    }

    /// Returns `state` after verifying it against its file when `verify_state` is on
    ///
    /// # Errors
    ///
    /// Returns the errors of [`State::verify`] when [`verify_state`](Self::verify_state) is on
    pub(crate) fn verified(&self, state: State) -> Result<State> {
        if self.verify_state {
            state.verify()?;
        }

        Ok(state)
    }

    /// Returns a temporary path, stamped with the current time, for writing the state of `name`
    ///
    /// The current time in nanoseconds goes before [`STATE_FILE_EXTENSION`] and
    /// [`TMP_FILE_EXTENSION`] goes after it
    ///
    /// # Errors
    ///
    /// Returns the errors of [`locate`](StateRegistry::locate)
    fn tmp(&self, name: &str) -> Result<PathBuf> {
        let stamp = UtcDateTime::now().unix_timestamp_nanos().to_string();
        let extensions = &[&stamp, STATE_FILE_EXTENSION, TMP_FILE_EXTENSION];

        self.locate(name, extensions)
    }

    /// Joins `name` under the state directory, appends `extensions` and rejects aliases
    ///
    /// # Errors
    ///
    /// Returns [`Error::Name`] when `name` is not a valid state name and [`Error::Alias`] when a
    /// part of it is a symlink or a case or Unicode alias
    fn locate(&self, name: &str, extensions: &[&str]) -> Result<PathBuf> {
        let mut path = path::scoped_join(&self.state_dir, name)?;

        for extension in extensions {
            path.add_extension(extension);
        }

        let resolved = path::resolves_in_place(&self.state_dir, &path);

        ensure!(resolved, Error::alias(name));

        Ok(path)
    }
}
