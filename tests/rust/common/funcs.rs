use std::{
    fs,
    path::{MAIN_SEPARATOR_STR, PathBuf},
};

use chrono::DateTime;
use preader::{FileMetadata, Result, State, StateData, StateRegistry, Timestamps};
use sha2::{Digest, Sha256};

use crate::common::{
    constants::{TEST_LINE, TEST_LINE_FINGERPRINT, TEST_STATE_NAME},
    interfaces::Item,
};

const STAMP: i64 = 1_700_000_000;

pub fn items<I, T>(iterator: I) -> Vec<T>
where
    I: Iterator<Item = Result<T>>,
{
    try_items(iterator).unwrap()
}

pub fn try_items<I, T>(iterator: I) -> Result<Vec<T>>
where
    I: Iterator<Item = Result<T>>,
{
    iterator.collect()
}

pub fn take<I, T>(iterator: &mut I, count: usize) -> Vec<T>
where
    I: Iterator<Item = Result<T>>,
{
    items(iterator.take(count))
}

pub fn consume<I, T>(iterator: &mut I, count: usize)
where
    I: Iterator<Item = Result<T>>,
{
    take(iterator, count);
}

pub fn texts(items: &[Vec<u8>]) -> Vec<String> {
    items.iter().map(|item| String::from_utf8(item.clone()).unwrap()).collect()
}

pub fn flatten<T: Item>(items: Vec<T>) -> Vec<Vec<u8>> {
    items.into_iter().map(Item::bytes).collect()
}

pub fn digest(content: &[u8]) -> String {
    hex::encode(Sha256::digest(content))
}

pub fn native(name: &str) -> String {
    name.replace('/', MAIN_SEPARATOR_STR)
}

pub fn names(registry: &StateRegistry) -> Vec<String> {
    let mut names: Vec<String> = registry.names().unwrap().map(Result::unwrap).collect();

    names.sort();
    names
}

pub fn state_data(path: PathBuf) -> StateData {
    let stamp = DateTime::from_timestamp(STAMP, 0).unwrap();

    StateData {
        name: TEST_STATE_NAME.to_owned(),
        file: FileMetadata {
            path,
            size: TEST_LINE.len() as u64,
            mtime: stamp,
            fingerprint: TEST_LINE_FINGERPRINT.to_owned(),
        },
        position: 7,
        timestamps: Timestamps {
            created_at: stamp,
            updated_at: stamp,
        },
        checksum: String::new(),
    }
}

pub fn tamper(state: &State) {
    let payload = state.path().unwrap();
    let patched = fs::read_to_string(&payload)
        .unwrap()
        .replace("\"position\": 0", "\"position\": 999");

    fs::write(&payload, patched).unwrap();
}
