use std::{
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::utils::hash;

pub fn fingerprint(path: &Path, bytes: u64) -> io::Result<String> {
    let mut buffer = Vec::new();

    File::open(path)?.take(bytes).read_to_end(&mut buffer)?;

    Ok(hash::sha256(&buffer))
}

pub fn write_atomic(path: &Path, tmp: &Path, content: impl AsRef<[u8]>) -> io::Result<()> {
    path.parent().map(fs::create_dir_all).transpose()?;

    let mut file = File::create_new(tmp)?;

    file.write_all(content.as_ref())
        .and_then(|()| file.sync_all())
        .and_then(|()| fs::rename(tmp, path))
        .inspect_err(|_| drop(fs::remove_file(tmp)))
}

pub fn starts_mid_item(file: &File, position: u64, boundary: u8) -> io::Result<bool> {
    let Some(previous) = position.checked_sub(1) else {
        return Ok(false);
    };

    let mut handle = file;
    let mut byte = [0u8; 1];

    handle.seek(SeekFrom::Start(previous))?;

    let read_count = handle.read(&mut byte)?;

    handle.seek(SeekFrom::Start(position))?;

    Ok(read_count == 1 && byte[0] != boundary)
}
