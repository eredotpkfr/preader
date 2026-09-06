use preader::{Config, IteratorBuild, IteratorRead};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    fixtures::tmp_dir,
    funcs::{reader, write},
};

const CONTENT: &[u8] = b"abcdef";

#[rstest]
fn read_yields_every_byte(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader.bytes(&path).build().unwrap();
    let mut collected = Vec::new();

    while let Some(byte) = bytes.read().unwrap() {
        collected.push(byte);
    }

    assert_eq!(collected, CONTENT);
}

#[rstest]
#[case::start(2, u64::MAX, 0, u64::MAX, b"cdef")]
#[case::end(0, 3, 0, u64::MAX, b"abc")]
#[case::skip(0, u64::MAX, 4, u64::MAX, b"ef")]
#[case::limit(0, u64::MAX, 0, 2, b"ab")]
#[case::start_and_skip(1, u64::MAX, 2, u64::MAX, b"def")]
#[case::the_tighter_of_end_and_limit(0, 2, 0, 5, b"ab")]
fn options_narrow_the_output(
    tmp_dir: TempDir,
    #[case] start: u64,
    #[case] end: u64,
    #[case] skip: u64,
    #[case] limit: u64,
    #[case] expected: &[u8],
) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut bytes = reader
        .bytes(&path)
        .start(start)
        .end(end)
        .skip(skip)
        .limit(limit)
        .build()
        .unwrap();
    let mut collected = Vec::new();

    while let Some(byte) = bytes.read().unwrap() {
        collected.push(byte);
    }

    assert_eq!(collected, expected);
}

#[rstest]
fn read_yields_nothing_for_an_empty_file(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"");
    let mut bytes = reader.bytes(&path).build().unwrap();

    assert_eq!(bytes.read().unwrap(), None);
}

#[rstest]
fn read_stays_exhausted_after_the_end(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", b"a");
    let mut bytes = reader.bytes(&path).build().unwrap();

    assert_eq!(bytes.read().unwrap(), Some(b'a'));
    assert_eq!(bytes.read().unwrap(), None);
    assert_eq!(bytes.read().unwrap(), None);
}

#[rstest]
fn the_iterator_is_a_std_iterator(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let collected: Result<Vec<u8>, _> = reader.bytes(&path).build().unwrap().collect();

    assert_eq!(collected.unwrap(), CONTENT);

    let counted = reader
        .bytes(&path)
        .build()
        .unwrap()
        .filter(|byte| matches!(byte, Ok(b'a')))
        .count();

    assert_eq!(counted, 1);
}

#[rstest]
fn a_for_loop_yields_owned_bytes(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut collected = Vec::new();

    for byte in reader.bytes(&path).build().unwrap() {
        collected.push(byte.unwrap());
    }

    assert_eq!(collected, CONTENT);
}
