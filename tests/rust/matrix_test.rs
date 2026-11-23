use rstest::rstest;
use rstest_reuse::apply;

use crate::common::{
    constants::TEST_TIMEOUT,
    fixtures::{sandbox, seed},
    matrix::{asserts::assert_reference, reference, reference::Shape, strategies},
    property::check,
    sandbox::Sandbox,
    seed::Seed,
    templates::delimiter::delimiters,
};

#[rstest]
#[timeout(TEST_TIMEOUT)]
fn bytes_match_the_reference(sandbox: Sandbox, seed: Seed) {
    check(seed, strategies::bytes(), |(content, options, capacity)| {
        let path = sandbox.file(&content);
        let built = sandbox.capped(capacity).bytes(&path).options(options).build();

        assert_reference(built, options, reference::bytes(&content, options))
    });
}

#[rstest]
#[timeout(TEST_TIMEOUT)]
fn chunks_match_the_reference(
    sandbox: Sandbox,
    seed: Seed,
    #[values(false, true)] drop_partial: bool,
) {
    check(
        seed,
        strategies::chunks(),
        |(content, options, size, capacity)| {
            let path = sandbox.file(&content);
            let built = sandbox
                .capped(capacity)
                .chunks(&path)
                .options(options)
                .size(size)
                .drop_partial(drop_partial)
                .build();

            assert_reference(
                built,
                options,
                reference::chunks(&content, size, options, drop_partial),
            )
        },
    );
}

#[rstest]
#[timeout(TEST_TIMEOUT)]
fn lines_match_the_reference(
    sandbox: Sandbox,
    seed: Seed,
    #[values(false, true)] keep: bool,
    #[values(false, true)] skip_empty: bool,
    #[values(false, true)] align: bool,
) {
    let shape = Shape {
        keep,
        skip_empty,
        align,
    };

    check(seed, strategies::lines(), |(content, options, capacity)| {
        let path = sandbox.file(&content);
        let built = sandbox
            .capped(capacity)
            .lines(&path)
            .options(options)
            .keepends(keep)
            .skip_empty(skip_empty)
            .align(align)
            .build();

        assert_reference(built, options, reference::lines(&content, shape, options))
    });
}

#[apply(delimiters)]
#[timeout(TEST_TIMEOUT)]
fn segments_match_the_reference(
    sandbox: Sandbox,
    seed: Seed,
    #[values(false, true)] keep: bool,
    #[values(false, true)] skip_empty: bool,
    #[values(false, true)] align: bool,
    #[case] delimiter: u8,
) {
    let shape = Shape {
        keep,
        skip_empty,
        align,
    };

    check(
        seed,
        strategies::segments(delimiter),
        |(content, options, capacity)| {
            let path = sandbox.file(&content);
            let built = sandbox
                .capped(capacity)
                .segments(&path)
                .options(options)
                .delimiter(delimiter)
                .keep(keep)
                .skip_empty(skip_empty)
                .align(align)
                .build();

            assert_reference(
                built,
                options,
                reference::segments(&content, shape, options, delimiter),
            )
        },
    );
}
