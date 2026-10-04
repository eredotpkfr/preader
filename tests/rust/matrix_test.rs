use std::{str, time::Duration};

use preader::{DEFAULT_DELIMITER, Error, IteratorBuild, IteratorOptions};
use rstest::rstest;

const BUDGET: Duration = Duration::from_secs(60);

use crate::common::{
    expected::{self, Split},
    fixtures::sandbox,
    funcs::{flatten, items, try_items},
    matrix::{
        BLANK_LINES, BLANK_SEGMENTS, CRLF_LINES, EMPTY, LINES, SEGMENTS, UNTERMINATED_LINES,
        UNTERMINATED_SEGMENTS, WHOLE_SEGMENT, shapes, sizings, windows,
    },
    sandbox::Sandbox,
};

fn assert_bounds<T: std::fmt::Debug>(built: Result<T, Error>, options: IteratorOptions) {
    let error = built.unwrap_err();
    let reported = Error::InvalidRange {
        start: options.start,
        end: options.end,
    };

    assert_eq!(error.to_string(), reported.to_string(), "{options:?}");
}

#[rstest]
#[timeout(BUDGET)]
fn bytes_match_the_reference(
    sandbox: Sandbox,
    #[values(EMPTY, LINES, UNTERMINATED_LINES, CRLF_LINES, BLANK_LINES)] content: &[u8],
) {
    let path = sandbox.file(content);
    let reader = sandbox.reader();

    for options in windows() {
        let built = reader.bytes(&path).options(options).build();

        if options.start > options.end {
            assert_bounds(built, options);

            continue;
        }

        assert_eq!(
            items(built.unwrap()),
            expected::bytes(content, options),
            "{options:?}"
        );
    }
}

#[rstest]
#[timeout(BUDGET)]
fn chunks_match_the_reference(
    sandbox: Sandbox,
    #[values(EMPTY, LINES, UNTERMINATED_LINES, CRLF_LINES, BLANK_LINES)] content: &[u8],
) {
    let path = sandbox.file(content);
    let reader = sandbox.reader();

    for options in windows() {
        for (size, drop_partial) in sizings() {
            let built = reader
                .chunks(&path)
                .options(options)
                .size(size)
                .drop_partial(drop_partial)
                .build();

            if options.start > options.end {
                assert_bounds(built, options);

                continue;
            }

            assert_eq!(
                items(built.unwrap()),
                expected::chunks(content, size, options, drop_partial),
                "{options:?} size={size} drop_partial={drop_partial}"
            );
        }
    }
}

#[rstest]
#[timeout(BUDGET)]
fn lines_match_the_reference(
    sandbox: Sandbox,
    #[values(EMPTY, LINES, UNTERMINATED_LINES, CRLF_LINES, BLANK_LINES)] content: &[u8],
) {
    let path = sandbox.file(content);
    let reader = sandbox.reader();

    for options in windows() {
        for shape in shapes() {
            let built = reader
                .lines(&path)
                .options(options)
                .keepends(shape.keep)
                .skip_empty(shape.skip_empty)
                .align(shape.align)
                .build();

            if options.start > options.end {
                assert_bounds(built, options);

                continue;
            }

            assert_eq!(
                flatten(try_items(built.unwrap()).unwrap()),
                expected::split(content, &Split::lines(shape), options),
                "content={:?} {options:?} {shape:?}",
                str::from_utf8(content).unwrap()
            );
        }
    }
}

#[rstest]
#[timeout(BUDGET)]
fn segments_match_the_reference(
    sandbox: Sandbox,
    #[values(EMPTY, SEGMENTS, UNTERMINATED_SEGMENTS, BLANK_SEGMENTS, WHOLE_SEGMENT)]
    content: &[u8],
) {
    let path = sandbox.file(content);
    let reader = sandbox.reader();

    for options in windows() {
        for shape in shapes() {
            let built = reader
                .delimiter(&path)
                .options(options)
                .character(DEFAULT_DELIMITER)
                .keep(shape.keep)
                .skip_empty(shape.skip_empty)
                .align(shape.align)
                .build();

            if options.start > options.end {
                assert_bounds(built, options);

                continue;
            }

            assert_eq!(
                items(built.unwrap()),
                expected::split(content, &Split::segments(shape), options),
                "content={:?} {options:?} {shape:?}",
                str::from_utf8(content).unwrap()
            );
        }
    }
}
