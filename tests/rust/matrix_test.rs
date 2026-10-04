use preader::{DEFAULT_DELIMITER, IteratorBuild};
use rstest::rstest;

use crate::common::{
    constants::TEST_TIMEOUT,
    fixtures::sandbox,
    funcs::{items, try_items},
    matrix::{
        BLANK_LINES, BLANK_SEGMENTS, CRLF_LINES, EMPTY, LINES, SEGMENTS, UNTERMINATED_LINES,
        UNTERMINATED_SEGMENTS, WHOLE_SEGMENT, assert_bounds, expected, shapes, sizings, windows,
    },
    sandbox::Sandbox,
};

#[rstest]
#[timeout(TEST_TIMEOUT)]
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
#[timeout(TEST_TIMEOUT)]
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
#[timeout(TEST_TIMEOUT)]
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
                try_items(built.unwrap()).unwrap(),
                expected::lines(content, shape, options),
                "{options:?} {shape:?}"
            );
        }
    }
}

#[rstest]
#[timeout(TEST_TIMEOUT)]
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
                expected::segments(content, shape, options),
                "{options:?} {shape:?}"
            );
        }
    }
}
