use preader::{Config, IteratorBuild, IteratorRead};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    fixtures::tmp_dir,
    funcs::{reader, write},
};

const CONTENT: &[u8] = b"abcdefgh";

fn chunks(tmp_dir: &TempDir, size: usize, drop_partial: bool) -> Vec<Vec<u8>> {
    let reader = reader(tmp_dir, Config::default());
    let path = write(tmp_dir, "data.bin", CONTENT);
    let mut iterator = reader.chunks(&path).size(size).drop_partial(drop_partial).build().unwrap();
    let mut collected = Vec::new();

    while let Some(chunk) = iterator.read().unwrap() {
        collected.push(chunk.to_vec());
    }

    collected
}

#[rstest]
fn read_keeps_a_short_final_chunk(tmp_dir: TempDir) {
    assert_eq!(
        chunks(&tmp_dir, 3, false),
        [b"abc".to_vec(), b"def".to_vec(), b"gh".to_vec()]
    );
}

#[rstest]
fn drop_partial_discards_a_short_final_chunk(tmp_dir: TempDir) {
    assert_eq!(
        chunks(&tmp_dir, 3, true),
        [b"abc".to_vec(), b"def".to_vec()]
    );
}

#[rstest]
fn drop_partial_keeps_an_exactly_divisible_tail(tmp_dir: TempDir) {
    assert_eq!(
        chunks(&tmp_dir, 4, true),
        [b"abcd".to_vec(), b"efgh".to_vec()]
    );
}

#[rstest]
fn a_chunk_larger_than_the_file_yields_one_chunk(tmp_dir: TempDir) {
    assert_eq!(chunks(&tmp_dir, 1024, false), [CONTENT.to_vec()]);
}

#[rstest]
fn a_zero_sized_chunk_yields_nothing(tmp_dir: TempDir) {
    assert!(chunks(&tmp_dir, 0, false).is_empty());
}

#[rstest]
fn a_chunk_of_one_matches_the_byte_iterator(tmp_dir: TempDir) {
    let expected: Vec<Vec<u8>> = CONTENT.iter().map(|byte| vec![*byte]).collect();

    assert_eq!(chunks(&tmp_dir, 1, false), expected);
}

#[rstest]
fn skip_counts_chunks_not_bytes(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut iterator = reader.chunks(&path).size(2).skip(2).build().unwrap();
    let mut collected = Vec::new();

    while let Some(chunk) = iterator.read().unwrap() {
        collected.push(chunk.to_vec());
    }

    assert_eq!(collected, [b"ef".to_vec(), b"gh".to_vec()]);
}

#[rstest]
fn drop_partial_discards_a_chunk_cut_short_by_end(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut iterator = reader.chunks(&path).size(3).end(5).drop_partial(true).build().unwrap();
    let mut collected = Vec::new();

    while let Some(chunk) = iterator.read().unwrap() {
        collected.push(chunk.to_vec());
    }

    assert_eq!(collected, [b"abc".to_vec()]);
    assert_eq!(iterator.state().position, 3);
}

#[rstest]
fn a_for_loop_yields_owned_chunks(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", CONTENT);
    let mut collected = Vec::new();

    for chunk in reader.chunks(&path).size(3).build().unwrap() {
        collected.push(chunk.unwrap());
    }

    assert_eq!(
        collected,
        [b"abc".to_vec(), b"def".to_vec(), b"gh".to_vec()]
    );
}
