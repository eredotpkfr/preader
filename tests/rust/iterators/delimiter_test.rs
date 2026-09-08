use preader::{Config, IteratorBuild, IteratorRead};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    fixtures::tmp_dir,
    funcs::{reader, write},
};

const CONTENT: &[u8] = b"a,b,,c";

fn segments(tmp_dir: &TempDir, content: &[u8], keep: bool, skip_empty: bool) -> Vec<Vec<u8>> {
    let reader = reader(tmp_dir, Config::default());
    let path = write(tmp_dir, "data.csv", content);
    let mut iterator = reader.delimiter(&path).keep(keep).skip_empty(skip_empty).build().unwrap();
    let mut collected = Vec::new();

    while let Some(segment) = iterator.read().unwrap() {
        collected.push(segment.to_vec());
    }

    collected
}

#[rstest]
fn read_splits_on_the_default_comma(tmp_dir: TempDir) {
    let expected = [b"a".to_vec(), b"b".to_vec(), Vec::new(), b"c".to_vec()];

    assert_eq!(segments(&tmp_dir, CONTENT, false, false), expected);
}

#[rstest]
fn keep_preserves_the_delimiter(tmp_dir: TempDir) {
    let expected = [b"a,".to_vec(), b"b,".to_vec(), b",".to_vec(), b"c".to_vec()];

    assert_eq!(segments(&tmp_dir, CONTENT, true, false), expected);
}

#[rstest]
fn skip_empty_drops_blank_segments(tmp_dir: TempDir) {
    let expected = [b"a".to_vec(), b"b".to_vec(), b"c".to_vec()];

    assert_eq!(segments(&tmp_dir, CONTENT, false, true), expected);
}

#[rstest]
fn skip_empty_judges_blankness_without_the_delimiter(tmp_dir: TempDir) {
    let expected = [b"a,".to_vec(), b"b,".to_vec(), b"c".to_vec()];

    assert_eq!(segments(&tmp_dir, CONTENT, true, true), expected);
}

#[rstest]
fn a_file_without_the_delimiter_yields_one_segment(tmp_dir: TempDir) {
    assert_eq!(segments(&tmp_dir, b"abc", false, false), [b"abc".to_vec()]);
}

#[rstest]
fn a_trailing_delimiter_yields_no_extra_segment(tmp_dir: TempDir) {
    assert_eq!(
        segments(&tmp_dir, b"a,b,", false, false),
        [b"a".to_vec(), b"b".to_vec()]
    );
}

#[rstest]
#[case::null(b'\0', b"a\0b\0c")]
#[case::newline(b'\n', b"a\nb\nc")]
#[case::high_byte(b'\xff', b"a\xffb\xffc")]
fn character_selects_the_separator(
    tmp_dir: TempDir,
    #[case] character: u8,
    #[case] content: &[u8],
) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.bin", content);
    let mut iterator = reader.delimiter(&path).character(character).build().unwrap();
    let mut collected = Vec::new();

    while let Some(segment) = iterator.read().unwrap() {
        collected.push(segment.to_vec());
    }

    assert_eq!(collected, [b"a".to_vec(), b"b".to_vec(), b"c".to_vec()]);
}

#[rstest]
fn align_skips_a_segment_the_window_starts_inside(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.csv", b"aaa,bbb,ccc");
    let mut aligned = reader.delimiter(&path).start(1).align(true).build().unwrap();
    let mut partial = reader.delimiter(&path).start(1).build().unwrap();

    assert_eq!(aligned.read().unwrap(), Some(&b"bbb"[..]));
    assert_eq!(partial.read().unwrap(), Some(&b"aa"[..]));
}

#[rstest]
fn keep_does_not_change_the_position(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.csv", CONTENT);
    let mut stripped = reader.delimiter(&path).build().unwrap();
    let mut kept = reader.delimiter(&path).keep(true).build().unwrap();

    while stripped.read().unwrap().is_some() {}
    while kept.read().unwrap().is_some() {}

    assert_eq!(stripped.state().position, kept.state().position);
}

#[rstest]
fn skip_counts_segments_not_bytes(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.csv", b"a,b,c,d");
    let mut iterator = reader.delimiter(&path).skip(2).build().unwrap();
    let mut collected = Vec::new();

    while let Some(segment) = iterator.read().unwrap() {
        collected.push(segment.to_vec());
    }

    assert_eq!(collected, [b"c".to_vec(), b"d".to_vec()]);
}

#[rstest]
fn a_resumed_read_ignores_align(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.csv", b"aaa,bbb,ccc");
    let mut first = reader.bytes(&path).limit(1).build().unwrap();

    while first.read().unwrap().is_some() {}

    let mut saved = first.state().clone();

    saved.save().unwrap();

    let mut resumed = reader.delimiter(&path).state(saved).align(true).build().unwrap();
    let mut collected = Vec::new();

    while let Some(segment) = resumed.read().unwrap() {
        collected.push(segment.to_vec());
    }

    assert_eq!(
        collected,
        [b"aa".to_vec(), b"bbb".to_vec(), b"ccc".to_vec()]
    );
}

#[rstest]
fn a_for_loop_yields_owned_segments(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.csv", CONTENT);
    let mut collected = Vec::new();

    for segment in reader.delimiter(&path).build().unwrap() {
        collected.push(segment.unwrap());
    }

    assert_eq!(
        collected,
        [b"a".to_vec(), b"b".to_vec(), Vec::new(), b"c".to_vec()]
    );
}
