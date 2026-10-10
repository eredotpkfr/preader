use preader::{Error, IteratorBuild, IteratorOptions, Skip};
use rstest::rstest;

use crate::common::{fixtures::sandbox, macros::asserts::assert_err_is, sandbox::Sandbox};

const FILE_SIZE: u64 = 100;

fn range(start: u64, end: u64) -> IteratorOptions {
    IteratorOptions {
        start,
        end,
        ..IteratorOptions::default()
    }
}

#[rstest]
fn defaults_are_unbounded() {
    let options = IteratorOptions::default();

    assert_eq!(options.start, 0);
    assert_eq!(options.end, u64::MAX);
    assert_eq!(options.skip, 0);
    assert_eq!(options.limit, u64::MAX);
}

#[rstest]
fn compares_by_value() {
    let options = IteratorOptions {
        start: 10,
        end: 100,
        ..IteratorOptions::default()
    };

    assert_eq!(
        options,
        IteratorOptions {
            start: 10,
            end: 100,
            ..IteratorOptions::default()
        }
    );

    assert_ne!(
        options,
        IteratorOptions {
            start: 10,
            end: 101,
            ..IteratorOptions::default()
        }
    );
}

#[rstest]
#[case::zero_length(range(0, 0))]
#[case::whole_range(range(0, u64::MAX))]
#[case::equal_bounds(range(10, 10))]
fn build_accepts_an_ordered_range(sandbox: Sandbox, #[case] options: IteratorOptions) {
    let path = sandbox.line_file();

    assert!(sandbox.reader().bytes(path).options(options).build().is_ok());
}

#[rstest]
#[case::one_apart(range(1, 0))]
#[case::far_apart(range(100, 10))]
#[case::maximum_start(range(u64::MAX, 0))]
fn build_fails_when_the_range_is_inverted(sandbox: Sandbox, #[case] options: IteratorOptions) {
    let path = sandbox.line_file();
    let reader = sandbox.reader();
    let (start, end) = (options.start, options.end);

    assert_err_is!(
        reader.bytes(&path).options(options).build(),
        Error::InvalidRange { start: from, end: to } if *from == start && *to == end
    );

    assert_err_is!(
        reader.bytes(&path).options(options).build(),
        Error::InvalidRange { start: s, end: e } if *s == start && *e == end
    );
}

#[rstest]
#[case::unread_file(IteratorOptions::default(), 0, 0, true)]
#[case::start_inside_the_file(IteratorOptions { start: 10, ..IteratorOptions::default() }, 0, 10, true)]
#[case::resume_at_the_start(IteratorOptions { start: 10, ..IteratorOptions::default() }, 10, 10, true)]
#[case::resume_past_the_start(IteratorOptions { start: 10, ..IteratorOptions::default() }, 40, 40, false)]
#[case::resume_before_the_start(IteratorOptions { start: 40, ..IteratorOptions::default() }, 10, 40, true)]
#[case::fully_consumed(IteratorOptions::default(), FILE_SIZE, FILE_SIZE, false)]
#[case::resume_past_the_end(IteratorOptions { end: 50, ..IteratorOptions::default() }, 80, 80, false)]
#[case::start_past_the_file(IteratorOptions { start: 150, ..IteratorOptions::default() }, 0, FILE_SIZE, true)]
#[case::start_at_the_file_end(
    IteratorOptions { start: FILE_SIZE, ..IteratorOptions::default() },
    FILE_SIZE,
    FILE_SIZE,
    false
)]
fn window_resolves_the_position(
    #[case] options: IteratorOptions,
    #[case] position: u64,
    #[case] expected: u64,
    #[case] skipping: bool,
) {
    let window = options.window(position, FILE_SIZE, Skip::Bytes(0));

    assert_eq!(window.position, expected);
    assert_eq!(window.skipping.is_some(), skipping);
}

#[rstest]
#[case::unbounded(u64::MAX, FILE_SIZE)]
#[case::inside_the_file(50, 50)]
#[case::at_the_file_size(FILE_SIZE, FILE_SIZE)]
#[case::past_the_file_size(FILE_SIZE + 1, FILE_SIZE)]
fn window_clamps_the_end_to_the_size(#[case] end: u64, #[case] expected: u64) {
    let options = IteratorOptions {
        end,
        ..IteratorOptions::default()
    };

    assert_eq!(options.window(0, FILE_SIZE, Skip::Bytes(0)).end, expected);
}

#[rstest]
#[case::no_skip(0, 10)]
#[case::inside_the_file(15, 25)]
#[case::at_the_end(90, FILE_SIZE)]
#[case::past_the_end(FILE_SIZE, FILE_SIZE)]
#[case::maximum_skip(u64::MAX, FILE_SIZE)]
fn window_folds_skipped_bytes_into_the_start(#[case] bytes: u64, #[case] expected: u64) {
    let options = IteratorOptions {
        start: 10,
        ..IteratorOptions::default()
    };

    assert_eq!(
        options.window(0, FILE_SIZE, Skip::Bytes(bytes)).position,
        expected
    );
}

#[rstest]
#[case::from_the_start(0, Some(3))]
#[case::mid_file(40, None)]
fn window_carries_skipped_items(#[case] position: u64, #[case] expected: Option<u64>) {
    let skip = Skip::Items {
        count: 3,
        boundary: None,
    };

    assert_eq!(
        IteratorOptions::default().window(position, FILE_SIZE, skip).skipping,
        expected
    );
}

#[rstest]
fn window_resolves_an_empty_file() {
    let window = IteratorOptions::default().window(0, 0, Skip::Bytes(0));

    assert!(window.skipping.is_none());

    assert_eq!(window.position, 0);
    assert_eq!(window.end, 0);
}
