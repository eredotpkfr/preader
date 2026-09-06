use preader::{Config, IteratorBuild, IteratorRead};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    fixtures::tmp_dir,
    funcs::{reader, write},
};

const CONTENT: &[u8] = b"alpha\n\nbeta\ngamma\n";

fn lines(tmp_dir: &TempDir, content: &[u8], keepends: bool, skip_empty: bool) -> Vec<String> {
    let reader = reader(tmp_dir, Config::default());
    let path = write(tmp_dir, "data.txt", content);
    let mut iterator =
        reader.lines(&path).keepends(keepends).skip_empty(skip_empty).build().unwrap();
    let mut collected = Vec::new();

    while let Some(line) = iterator.read().unwrap() {
        collected.push(line.to_owned());
    }

    collected
}

#[rstest]
fn read_strips_the_line_ending(tmp_dir: TempDir) {
    assert_eq!(
        lines(&tmp_dir, CONTENT, false, false),
        ["alpha", "", "beta", "gamma"]
    );
}

#[rstest]
fn keepends_preserves_the_line_ending(tmp_dir: TempDir) {
    assert_eq!(
        lines(&tmp_dir, CONTENT, true, false),
        ["alpha\n", "\n", "beta\n", "gamma\n"]
    );
}

#[rstest]
fn skip_empty_drops_blank_lines(tmp_dir: TempDir) {
    assert_eq!(
        lines(&tmp_dir, CONTENT, false, true),
        ["alpha", "beta", "gamma"]
    );
}

#[rstest]
fn skip_empty_judges_blankness_after_trimming(tmp_dir: TempDir) {
    assert_eq!(
        lines(&tmp_dir, CONTENT, true, true),
        ["alpha\n", "beta\n", "gamma\n"]
    );
}

#[rstest]
fn crlf_strips_both_characters(tmp_dir: TempDir) {
    assert_eq!(
        lines(&tmp_dir, b"foo\r\nbar\r\n", false, false),
        ["foo", "bar"]
    );
}

#[rstest]
fn keepends_preserves_the_full_crlf_terminator(tmp_dir: TempDir) {
    assert_eq!(
        lines(&tmp_dir, b"foo\r\nbar\r\n", true, false),
        ["foo\r\n", "bar\r\n"]
    );
}

#[rstest]
fn a_lone_carriage_return_is_not_a_separator(tmp_dir: TempDir) {
    assert_eq!(lines(&tmp_dir, b"foo\rbar", false, false), ["foo\rbar"]);
}

#[rstest]
fn keepends_leaves_the_last_line_unterminated(tmp_dir: TempDir) {
    assert_eq!(lines(&tmp_dir, b"foo\nbar", true, false), ["foo\n", "bar"]);
}

#[rstest]
fn read_fails_on_invalid_utf8(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", b"\xff\xfe\n");
    let mut iterator = reader.lines(&path).build().unwrap();

    assert!(iterator.read().unwrap_err().to_string().contains("valid UTF-8"));
}

#[rstest]
fn align_skips_a_line_the_window_starts_inside(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", CONTENT);
    let mut aligned = reader.lines(&path).start(2).align(true).build().unwrap();
    let mut partial = reader.lines(&path).start(2).build().unwrap();

    assert_eq!(aligned.read().unwrap(), Some(""));
    assert_eq!(partial.read().unwrap(), Some("pha"));
}

#[rstest]
fn align_is_a_no_op_on_a_line_boundary(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", CONTENT);
    let mut aligned = reader.lines(&path).start(6).align(true).build().unwrap();

    assert_eq!(aligned.read().unwrap(), Some(""));
}

#[rstest]
fn align_and_skip_combine(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", CONTENT);
    let mut iterator = reader.lines(&path).start(2).align(true).skip(1).build().unwrap();

    assert_eq!(iterator.read().unwrap(), Some("beta"));
}

#[rstest]
fn a_line_crossing_the_end_is_yielded_whole(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", CONTENT);
    let mut iterator = reader.lines(&path).end(8).build().unwrap();
    let mut collected = Vec::new();

    while let Some(line) = iterator.read().unwrap() {
        collected.push(line.to_owned());
    }

    assert_eq!(collected, ["alpha", "", "beta"]);
    assert_eq!(iterator.state().position, 12);
}

#[rstest]
fn keepends_does_not_change_the_position(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", CONTENT);
    let mut stripped = reader.lines(&path).build().unwrap();
    let mut kept = reader.lines(&path).keepends(true).build().unwrap();

    while stripped.read().unwrap().is_some() {}
    while kept.read().unwrap().is_some() {}

    assert_eq!(stripped.state().position, kept.state().position);
}

#[rstest]
fn skip_counts_lines_not_bytes(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", b"a\nb\nc\nd\n");
    let mut iterator = reader.lines(&path).skip(2).build().unwrap();
    let mut collected = Vec::new();

    while let Some(line) = iterator.read().unwrap() {
        collected.push(line.to_owned());
    }

    assert_eq!(collected, ["c", "d"]);
}

#[rstest]
fn a_resumed_read_ignores_align(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", CONTENT);
    let mut first = reader.bytes(&path).limit(3).build().unwrap();

    while first.read().unwrap().is_some() {}

    let mut saved = first.state().clone();

    saved.save().unwrap();

    let mut resumed = reader.lines(&path).state(saved).start(2).align(true).build().unwrap();
    let mut collected = Vec::new();

    while let Some(line) = resumed.read().unwrap() {
        collected.push(line.to_owned());
    }

    // The window opens mid-line, but a resumed run already consumed its skips.
    assert_eq!(collected, ["ha", "", "beta", "gamma"]);
}

#[rstest]
fn a_for_loop_yields_owned_lines(tmp_dir: TempDir) {
    let reader = reader(&tmp_dir, Config::default());
    let path = write(&tmp_dir, "data.txt", CONTENT);
    let mut collected = Vec::new();

    for line in reader.lines(&path).build().unwrap() {
        collected.push(line.unwrap());
    }

    assert_eq!(collected, ["alpha", "", "beta", "gamma"]);
}
