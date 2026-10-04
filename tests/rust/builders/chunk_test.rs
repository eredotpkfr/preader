use preader::IteratorBuild;
use rstest::rstest;

use crate::common::{constants::TEST_ALPHABET, fixtures::sandbox, funcs::items, sandbox::Sandbox};

#[rstest]
fn the_default_size_comes_from_the_constant(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).build().unwrap());

    assert_eq!(read, [TEST_ALPHABET]);
}

#[rstest]
#[case::single_byte(1, 26)]
#[case::even_split(13, 2)]
#[case::uneven_split(10, 3)]
fn size_sets_the_item_length(sandbox: Sandbox, #[case] size: usize, #[case] expected: usize) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(size).build().unwrap());

    assert_eq!(read.len(), expected);
    assert_eq!(read[0].len(), size);
}

#[rstest]
fn the_last_size_wins(sandbox: Sandbox) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(sandbox.reader().chunks(&path).size(1).size(2).build().unwrap());

    assert_eq!(read[0].len(), 2);
}

#[rstest]
#[case::keeps_the_tail(false, 3)]
#[case::drops_the_tail(true, 2)]
fn drop_partial_decides_about_the_tail(
    sandbox: Sandbox,
    #[case] drop_partial: bool,
    #[case] expected: usize,
) {
    let path = sandbox.file(TEST_ALPHABET);
    let read = items(
        sandbox
            .reader()
            .chunks(&path)
            .size(10)
            .drop_partial(drop_partial)
            .build()
            .unwrap(),
    );

    assert_eq!(read.len(), expected);
}
