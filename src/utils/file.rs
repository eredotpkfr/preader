use std::{
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::utils::hash;

/// Returns the SHA-256 hex digest of the first `bytes` bytes of the file at `path`
///
/// A file shorter than `bytes` is hashed whole
///
/// # Errors
///
/// Returns an [`io::Error`] when the file cannot be opened or read
///
/// # Examples
///
/// ```
/// use preader::{file, hash};
///
/// let dir = tempfile::tempdir()?;
/// let path = dir.path().join("data.txt");
///
/// std::fs::write(&path, "foo\nbar\n")?;
///
/// let fingerprint = file::fingerprint(&path, 3)?;
///
/// assert_eq!(fingerprint, hash::sha256("foo"));
/// # Ok::<(), preader::Error>(())
/// ```
pub fn fingerprint(path: &Path, bytes: u64) -> io::Result<String> {
    let mut buffer = Vec::new();

    File::open(path)?.take(bytes).read_to_end(&mut buffer)?;

    Ok(hash::sha256(&buffer))
}

/// Writes `content` to `tmp` and renames it over `path`, so readers never see a partial file
///
/// The parent directory of `path` is created first. `tmp` must not exist yet and must be on the
/// same file system as `path`, since a rename cannot cross file systems. `tmp` is removed again
/// when writing, syncing or renaming fails
///
/// # Errors
///
/// Returns an [`io::Error`] when the parent directory cannot be created, when `tmp` already
/// exists or cannot be created, and when writing, syncing or renaming fails
///
/// # Examples
///
/// ```
/// use preader::file;
///
/// let dir = tempfile::tempdir()?;
/// let path = dir.path().join("states").join("job-1.state.json");
/// let tmp = dir.path().join("states").join("job-1.state.json.tmp");
///
/// file::write_atomic(&path, &tmp, "{}")?;
///
/// assert_eq!(std::fs::read_to_string(&path)?, "{}");
/// # Ok::<(), preader::Error>(())
/// ```
pub fn write_atomic(path: &Path, tmp: &Path, content: impl AsRef<[u8]>) -> io::Result<()> {
    path.parent().map(fs::create_dir_all).transpose()?;

    let mut file = File::create_new(tmp)?;

    file.write_all(content.as_ref())
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(tmp, path))
        .inspect_err(|_| drop(fs::remove_file(tmp)))
}

/// Returns whether `position` falls inside an item, that is, the byte before it is not `boundary`
///
/// Position 0 and a position past the end of the file are never inside an item. Every position
/// but 0 leaves the cursor of `file` at `position`
///
/// # Errors
///
/// Returns an [`io::Error`] when seeking or reading `file` fails
///
/// # Examples
///
/// ```
/// use std::fs::File;
///
/// use preader::file;
///
/// let dir = tempfile::tempdir()?;
/// let path = dir.path().join("data.txt");
///
/// std::fs::write(&path, "foo\nbar\n")?;
///
/// let inside = file::starts_mid_item(&File::open(&path)?, 5, b'\n')?;
///
/// assert!(inside);
/// # Ok::<(), preader::Error>(())
/// ```
pub fn starts_mid_item(mut file: &File, position: u64, boundary: u8) -> io::Result<bool> {
    let Some(previous) = position.checked_sub(1) else {
        return Ok(false);
    };

    let mut byte = [0u8; 1];

    file.seek(SeekFrom::Start(previous))?;

    let read_count = file.read(&mut byte)?;

    file.seek(SeekFrom::Start(position))?;

    Ok(read_count == 1 && byte[0] != boundary)
}
