use std::path::MAIN_SEPARATOR_STR;

use preader::{Result, StateRegistry};
use sha2::{Digest, Sha256};

pub trait Item {
    fn bytes(self) -> Vec<u8>;
}

impl Item for u8 {
    fn bytes(self) -> Vec<u8> {
        vec![self]
    }
}

impl Item for Vec<u8> {
    fn bytes(self) -> Vec<u8> {
        self
    }
}

impl Item for String {
    fn bytes(self) -> Vec<u8> {
        self.into_bytes()
    }
}

pub fn items<I, T: Item>(iterator: I) -> Vec<Vec<u8>>
where
    I: Iterator<Item = Result<T>>,
{
    try_items(iterator).unwrap()
}

pub fn try_items<I, T: Item>(iterator: I) -> Result<Vec<Vec<u8>>>
where
    I: Iterator<Item = Result<T>>,
{
    iterator.map(|item| item.map(Item::bytes)).collect()
}

pub fn take<I, T: Item>(iterator: &mut I, count: usize) -> Vec<Vec<u8>>
where
    I: Iterator<Item = Result<T>>,
{
    items(iterator.take(count))
}

pub fn texts(items: &[Vec<u8>]) -> Vec<String> {
    items.iter().map(|item| String::from_utf8(item.clone()).unwrap()).collect()
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
