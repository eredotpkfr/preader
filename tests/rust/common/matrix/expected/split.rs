use preader::{DEFAULT_DELIMITER, IteratorOptions};

use crate::common::matrix::{Shape, expected::cursor::cursor};

const LINE_BOUNDARY: u8 = b'\n';

struct Split {
    shape: Shape,
    carriage: bool,
    boundary: u8,
}

impl Split {
    fn lines(shape: Shape) -> Self {
        Self {
            boundary: LINE_BOUNDARY,
            carriage: true,
            shape,
        }
    }

    fn segments(shape: Shape) -> Self {
        Self {
            boundary: DEFAULT_DELIMITER,
            carriage: false,
            shape,
        }
    }

    fn trim<'a>(&self, raw: &'a [u8]) -> &'a [u8] {
        let mut trimmed = raw.strip_suffix(&[self.boundary]).unwrap_or(raw);

        while self.carriage
            && let Some(rest) = trimmed.strip_suffix(b"\r")
        {
            trimmed = rest;
        }

        trimmed
    }

    fn skipping(&self, content: &[u8], at: u64, end: u64, skip: u64) -> u64 {
        match (at < end, self.shape.align) {
            (false, _) => 0,
            (true, false) => skip,
            (true, true) => skip + misaligned(content, at, self.boundary),
        }
    }
}

fn split(content: &[u8], split: &Split, options: IteratorOptions) -> Vec<Vec<u8>> {
    let size = content.len() as u64;
    let (mut at, end) = cursor(size, options, 0);
    let mut skipping = split.skipping(content, at, end, options.skip);
    let (mut yielded, mut collected) = (0, Vec::new());

    while at < end && yielded < options.limit && at < size {
        let from = at as usize;
        let to = content[from..]
            .iter()
            .position(|byte| *byte == split.boundary)
            .map_or(size, |offset| at + offset as u64 + 1);
        let raw = &content[from..to as usize];

        at = to;

        if skipping > 0 {
            skipping -= 1;

            continue;
        }

        let trimmed = split.trim(raw);

        if split.shape.skip_empty && trimmed.is_empty() {
            continue;
        }

        yielded += 1;
        collected.push(if split.shape.keep {
            raw.to_vec()
        } else {
            trimmed.to_vec()
        });
    }

    collected
}

fn misaligned(content: &[u8], position: u64, boundary: u8) -> u64 {
    let Some(previous) = position.checked_sub(1) else {
        return 0;
    };

    u64::from(content.get(previous as usize).is_some_and(|byte| *byte != boundary))
}

pub fn lines(content: &[u8], shape: Shape, options: IteratorOptions) -> Vec<String> {
    split(content, &Split::lines(shape), options)
        .into_iter()
        .map(|line| String::from_utf8(line).unwrap())
        .collect()
}

pub fn segments(content: &[u8], shape: Shape, options: IteratorOptions) -> Vec<Vec<u8>> {
    split(content, &Split::segments(shape), options)
}
