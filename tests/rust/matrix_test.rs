use std::str;

use preader::{Error, IteratorBuild};
use rstest::rstest;

use crate::common::{
    constants::TEST_DELIMITER,
    fixtures::sandbox,
    funcs::{items, try_items},
    oracle::{self, ENDS, FLAGS, LIMITS, SIZES, SKIPS, SPLITS, STARTS, Split, TEXTS},
    sandbox::Sandbox,
};

fn assert_bounds<T: std::fmt::Debug>(built: Result<T, Error>, start: u64, end: u64) {
    let error = built.unwrap_err();

    assert!(
        matches!(error, Error::InvalidRange { start: from, end: to } if from == start && to == end),
        "start={start} end={end} produced {error}"
    );
}

#[rstest]
fn bytes_match_the_oracle(
    sandbox: Sandbox,
    #[values(TEXTS[0], TEXTS[1], TEXTS[2], TEXTS[3], TEXTS[4])] content: &[u8],
) {
    let path = sandbox.file(content);
    let reader = sandbox.reader();

    for start in STARTS {
        for end in ENDS {
            for skip in SKIPS {
                for limit in LIMITS {
                    let built =
                        reader.bytes(&path).start(start).end(end).skip(skip).limit(limit).build();

                    if start > end {
                        assert_bounds(built, start, end);

                        continue;
                    }

                    let read = items(built.unwrap()).concat();

                    assert_eq!(
                        read,
                        oracle::bytes(content, start, end, skip, limit),
                        "start={start} end={end} skip={skip} limit={limit}"
                    );
                }
            }
        }
    }
}

#[rstest]
fn chunks_match_the_oracle(
    sandbox: Sandbox,
    #[values(TEXTS[0], TEXTS[1], TEXTS[2], TEXTS[3], TEXTS[4])] content: &[u8],
) {
    let path = sandbox.file(content);
    let reader = sandbox.reader();

    for size in SIZES {
        for start in STARTS {
            for end in ENDS {
                for skip in SKIPS {
                    for limit in LIMITS {
                        for drop_partial in FLAGS {
                            let built = reader
                                .chunks(&path)
                                .size(size)
                                .start(start)
                                .end(end)
                                .skip(skip)
                                .limit(limit)
                                .drop_partial(drop_partial)
                                .build();

                            if start > end {
                                assert_bounds(built, start, end);

                                continue;
                            }

                            assert_eq!(
                                items(built.unwrap()),
                                oracle::chunks(
                                    content,
                                    size,
                                    start,
                                    end,
                                    skip,
                                    limit,
                                    drop_partial
                                ),
                                "size={size} start={start} end={end} skip={skip} \
                                 limit={limit} drop_partial={drop_partial}"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[rstest]
fn lines_match_the_oracle(
    sandbox: Sandbox,
    #[values(TEXTS[0], TEXTS[1], TEXTS[2], TEXTS[3], TEXTS[4])] content: &[u8],
) {
    let path = sandbox.file(content);
    let reader = sandbox.reader();

    for start in STARTS {
        for end in ENDS {
            for skip in SKIPS {
                for limit in LIMITS {
                    for keepends in FLAGS {
                        for skip_empty in FLAGS {
                            for align in FLAGS {
                                let built = reader
                                    .lines(&path)
                                    .start(start)
                                    .end(end)
                                    .skip(skip)
                                    .limit(limit)
                                    .keepends(keepends)
                                    .skip_empty(skip_empty)
                                    .align(align)
                                    .build();

                                if start > end {
                                    assert_bounds(built, start, end);

                                    continue;
                                }

                                let options = Split {
                                    boundary: b'\n',
                                    keep: keepends,
                                    skip_empty,
                                    align,
                                    carriage: true,
                                };

                                assert_eq!(
                                    try_items(built.unwrap()).unwrap(),
                                    oracle::split(content, &options, start, end, skip, limit),
                                    "content={:?} start={start} end={end} skip={skip} \
                                     limit={limit} keepends={keepends} \
                                     skip_empty={skip_empty} align={align}",
                                    str::from_utf8(content).unwrap()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[rstest]
fn segments_match_the_oracle(
    sandbox: Sandbox,
    #[values(SPLITS[0], SPLITS[1], SPLITS[2], SPLITS[3], SPLITS[4])] content: &[u8],
) {
    let path = sandbox.file(content);
    let reader = sandbox.reader();

    for start in STARTS {
        for end in ENDS {
            for skip in SKIPS {
                for limit in LIMITS {
                    for keep in FLAGS {
                        for skip_empty in FLAGS {
                            for align in FLAGS {
                                let built = reader
                                    .delimiter(&path)
                                    .character(TEST_DELIMITER)
                                    .start(start)
                                    .end(end)
                                    .skip(skip)
                                    .limit(limit)
                                    .keep(keep)
                                    .skip_empty(skip_empty)
                                    .align(align)
                                    .build();

                                if start > end {
                                    assert_bounds(built, start, end);

                                    continue;
                                }

                                let options = Split {
                                    boundary: TEST_DELIMITER,
                                    keep,
                                    skip_empty,
                                    align,
                                    carriage: false,
                                };

                                assert_eq!(
                                    items(built.unwrap()),
                                    oracle::split(content, &options, start, end, skip, limit),
                                    "content={:?} start={start} end={end} skip={skip} \
                                     limit={limit} keep={keep} skip_empty={skip_empty} \
                                     align={align}",
                                    str::from_utf8(content).unwrap()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
