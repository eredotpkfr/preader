use std::str;

use preader::{Config, Error, IteratorBuild, IteratorRead, PReader};
use rstest::rstest;
use tempfile::TempDir;

use crate::common::{
    fixtures::tmp_dir,
    funcs::{reader, write},
};

const STARTS: [u64; 4] = [0, 1, 3, 7];
const ENDS: [u64; 3] = [2, 5, u64::MAX];
const SKIPS: [u64; 3] = [0, 1, 3];
const LIMITS: [u64; 3] = [1, 2, u64::MAX];
const FLAGS: [bool; 2] = [false, true];

const ASCII: [&[u8]; 5] = [
    b"",
    b"alpha\n\nbeta\ngamma\n",
    b"alpha\n\nbeta\ngamma",
    b"foo\r\nbar\r\n",
    b"\n\n\n",
];

const SPLIT: [&[u8]; 5] = [b"", b"a,b,,c,", b"a,b,,c", b",,,", b"no-delimiter-here"];

fn window(size: u64, start: u64, end: u64, bytes: u64) -> (u64, u64) {
    let end = end.min(size);

    (start.saturating_add(bytes).min(end), end)
}

fn misaligned(content: &[u8], position: u64, boundary: u8) -> u64 {
    let Some(previous) = position.checked_sub(1) else {
        return 0;
    };

    u64::from(content.get(previous as usize).is_some_and(|byte| *byte != boundary))
}

#[expect(clippy::too_many_arguments)]
fn oracle_split(
    content: &[u8],
    boundary: u8,
    start: u64,
    end: u64,
    skip: u64,
    limit: u64,
    keep: bool,
    skip_empty: bool,
    align: bool,
    carriage: bool,
) -> Vec<Vec<u8>> {
    let size = content.len() as u64;
    let (mut cursor, end) = window(size, start, end, 0);
    let mut skipping = if cursor < end {
        skip + if align {
            misaligned(content, cursor, boundary)
        } else {
            0
        }
    } else {
        0
    };
    let (mut yielded, mut collected) = (0, Vec::new());

    while cursor < end && yielded < limit && cursor < size {
        let at = cursor as usize;
        let stop = content[at..]
            .iter()
            .position(|byte| *byte == boundary)
            .map_or(size, |offset| cursor + offset as u64 + 1);
        let raw = &content[at..stop as usize];

        cursor = stop;

        if skipping > 0 {
            skipping -= 1;

            continue;
        }

        let mut trimmed = raw.strip_suffix(&[boundary]).unwrap_or(raw);

        while carriage && let Some(rest) = trimmed.strip_suffix(b"\r") {
            trimmed = rest;
        }

        if skip_empty && trimmed.is_empty() {
            continue;
        }

        yielded += 1;
        collected.push(if keep { raw.to_vec() } else { trimmed.to_vec() });
    }

    collected
}

fn oracle_bytes(content: &[u8], start: u64, end: u64, skip: u64, limit: u64) -> Vec<u8> {
    let size = content.len() as u64;
    let (cursor, end) = window(size, start, end, skip);
    let stop = end.min(cursor.saturating_add(limit)).min(size);

    content[(cursor.min(size) as usize)..(stop.max(cursor).min(size) as usize)].to_vec()
}

fn oracle_chunks(
    content: &[u8],
    size_of: usize,
    start: u64,
    end: u64,
    skip: u64,
    limit: u64,
    drop_partial: bool,
) -> Vec<Vec<u8>> {
    let size = content.len() as u64;
    let stride = skip.saturating_mul(size_of as u64);
    let (mut cursor, end) = window(size, start, end, stride);
    let (mut yielded, mut collected) = (0, Vec::new());

    while cursor < end && yielded < limit {
        let max = (end - cursor).min(size_of as u64);

        if drop_partial && max < size_of as u64 {
            break;
        }

        let filled = max.min(size.saturating_sub(cursor));

        if filled == 0 || (drop_partial && filled < size_of as u64) {
            break;
        }

        collected.push(content[cursor as usize..(cursor + filled) as usize].to_vec());
        cursor += filled;
        yielded += 1;
    }

    collected
}

fn assert_bounds<T>(built: Result<T, Error>, start: u64, end: u64) {
    let error = built.err().unwrap();

    assert!(
        matches!(error, Error::InvalidRange { start: from, end: to } if from == start && to == end),
        "start={start} end={end} produced {error}"
    );
}

fn preader(tmp_dir: &TempDir) -> PReader {
    reader(tmp_dir, Config::default())
}

#[rstest]
fn lines_match_the_oracle_across_the_matrix(tmp_dir: TempDir) {
    let reader = preader(&tmp_dir);
    let mut cases = 0;

    for (index, content) in ASCII.iter().enumerate() {
        let path = write(&tmp_dir, &format!("line-{index}.txt"), content);

        for start in STARTS {
            for end in ENDS {
                for skip in SKIPS {
                    for limit in LIMITS {
                        for keepends in FLAGS {
                            for skip_empty in FLAGS {
                                for align in FLAGS {
                                    if start > end {
                                        assert_bounds(
                                            reader.lines(&path).start(start).end(end).build(),
                                            start,
                                            end,
                                        );

                                        cases += 1;

                                        continue;
                                    }

                                    let mut iterator = reader
                                        .lines(&path)
                                        .start(start)
                                        .end(end)
                                        .skip(skip)
                                        .limit(limit)
                                        .keepends(keepends)
                                        .skip_empty(skip_empty)
                                        .align(align)
                                        .build()
                                        .unwrap();
                                    let mut collected = Vec::new();

                                    while let Some(line) = iterator.read().unwrap() {
                                        collected.push(line.as_bytes().to_vec());
                                    }

                                    let expected = oracle_split(
                                        content, b'\n', start, end, skip, limit, keepends,
                                        skip_empty, align, true,
                                    );

                                    assert_eq!(
                                        collected,
                                        expected,
                                        "content={:?} start={start} end={end} skip={skip} \
                                         limit={limit} keepends={keepends} \
                                         skip_empty={skip_empty} align={align}",
                                        str::from_utf8(content).unwrap()
                                    );

                                    cases += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    assert_eq!(cases, 4320);
}

#[rstest]
fn delimiter_segments_match_the_oracle_across_the_matrix(tmp_dir: TempDir) {
    let reader = preader(&tmp_dir);
    let mut cases = 0;

    for (index, content) in SPLIT.iter().enumerate() {
        let path = write(&tmp_dir, &format!("split-{index}.bin"), content);

        for start in STARTS {
            for end in ENDS {
                for skip in SKIPS {
                    for limit in LIMITS {
                        for keep in FLAGS {
                            for skip_empty in FLAGS {
                                for align in FLAGS {
                                    if start > end {
                                        assert_bounds(
                                            reader.delimiter(&path).start(start).end(end).build(),
                                            start,
                                            end,
                                        );

                                        cases += 1;

                                        continue;
                                    }

                                    let mut iterator = reader
                                        .delimiter(&path)
                                        .character(b',')
                                        .start(start)
                                        .end(end)
                                        .skip(skip)
                                        .limit(limit)
                                        .keep(keep)
                                        .skip_empty(skip_empty)
                                        .align(align)
                                        .build()
                                        .unwrap();
                                    let mut collected = Vec::new();

                                    while let Some(segment) = iterator.read().unwrap() {
                                        collected.push(segment.to_vec());
                                    }

                                    let expected = oracle_split(
                                        content, b',', start, end, skip, limit, keep, skip_empty,
                                        align, false,
                                    );

                                    assert_eq!(
                                        collected, expected,
                                        "content={content:?} start={start} end={end} \
                                         skip={skip} limit={limit} keep={keep} \
                                         skip_empty={skip_empty} align={align}"
                                    );

                                    cases += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    assert_eq!(cases, 4320);
}

#[rstest]
fn bytes_match_the_oracle_across_the_matrix(tmp_dir: TempDir) {
    let reader = preader(&tmp_dir);
    let mut cases = 0;

    for (index, content) in ASCII.iter().enumerate() {
        let path = write(&tmp_dir, &format!("byte-{index}.bin"), content);

        for start in STARTS {
            for end in ENDS {
                for skip in SKIPS {
                    for limit in LIMITS {
                        if start > end {
                            assert_bounds(
                                reader.bytes(&path).start(start).end(end).build(),
                                start,
                                end,
                            );

                            cases += 1;

                            continue;
                        }

                        let mut iterator = reader
                            .bytes(&path)
                            .start(start)
                            .end(end)
                            .skip(skip)
                            .limit(limit)
                            .build()
                            .unwrap();
                        let mut collected = Vec::new();

                        while let Some(byte) = iterator.read().unwrap() {
                            collected.push(byte);
                        }

                        assert_eq!(
                            collected,
                            oracle_bytes(content, start, end, skip, limit),
                            "content={content:?} start={start} end={end} skip={skip} \
                             limit={limit}"
                        );

                        cases += 1;
                    }
                }
            }
        }
    }

    assert_eq!(cases, 540);
}

#[rstest]
fn chunks_match_the_oracle_across_the_matrix(tmp_dir: TempDir) {
    let reader = preader(&tmp_dir);
    let mut cases = 0;

    for (index, content) in ASCII.iter().enumerate() {
        let path = write(&tmp_dir, &format!("chunk-{index}.bin"), content);

        for size in [0_usize, 1, 3, 64] {
            for start in STARTS {
                for end in ENDS {
                    for skip in SKIPS {
                        for limit in LIMITS {
                            for drop_partial in FLAGS {
                                if start > end {
                                    assert_bounds(
                                        reader.chunks(&path).start(start).end(end).build(),
                                        start,
                                        end,
                                    );

                                    cases += 1;

                                    continue;
                                }

                                let mut iterator = reader
                                    .chunks(&path)
                                    .size(size)
                                    .start(start)
                                    .end(end)
                                    .skip(skip)
                                    .limit(limit)
                                    .drop_partial(drop_partial)
                                    .build()
                                    .unwrap();
                                let mut collected = Vec::new();

                                while let Some(chunk) = iterator.read().unwrap() {
                                    collected.push(chunk.to_vec());
                                }

                                assert_eq!(
                                    collected,
                                    oracle_chunks(
                                        content,
                                        size,
                                        start,
                                        end,
                                        skip,
                                        limit,
                                        drop_partial
                                    ),
                                    "content={content:?} size={size} start={start} end={end} \
                                     skip={skip} limit={limit} drop_partial={drop_partial}"
                                );

                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    assert_eq!(cases, 4320);
}
