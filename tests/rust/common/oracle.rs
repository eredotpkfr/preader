pub const STARTS: [u64; 4] = [0, 1, 3, 7];
pub const ENDS: [u64; 3] = [2, 5, u64::MAX];
pub const SKIPS: [u64; 3] = [0, 1, 3];
pub const LIMITS: [u64; 3] = [1, 2, u64::MAX];
pub const SIZES: [usize; 4] = [0, 1, 3, 64];
pub const FLAGS: [bool; 2] = [false, true];

pub const TEXTS: [&[u8]; 5] = [
    b"",
    b"line-0\n\nline-1\nline-2\n",
    b"line-0\n\nline-1\nline-2",
    b"line-0\r\nline-1\r\n",
    b"\n\n\n",
];

pub const SPLITS: [&[u8]; 5] = [
    b"",
    b"seg-0,seg-1,,seg-2,",
    b"seg-0,seg-1,,seg-2",
    b",,,",
    b"no-delimiter-here",
];

pub fn window(size: u64, start: u64, end: u64, bytes: u64) -> (u64, u64) {
    let end = end.min(size);

    (start.saturating_add(bytes).min(end), end)
}

pub fn misaligned(content: &[u8], position: u64, boundary: u8) -> u64 {
    let Some(previous) = position.checked_sub(1) else {
        return 0;
    };

    u64::from(content.get(previous as usize).is_some_and(|byte| *byte != boundary))
}

pub fn bytes(content: &[u8], start: u64, end: u64, skip: u64, limit: u64) -> Vec<u8> {
    let size = content.len() as u64;
    let (cursor, end) = window(size, start, end, skip);
    let stop = end.min(cursor.saturating_add(limit)).min(size);

    content[(cursor.min(size) as usize)..(stop.max(cursor).min(size) as usize)].to_vec()
}

pub fn chunks(
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

pub struct Split {
    pub boundary: u8,
    pub keep: bool,
    pub skip_empty: bool,
    pub align: bool,
    pub carriage: bool,
}

pub fn split(
    content: &[u8],
    options: &Split,
    start: u64,
    end: u64,
    skip: u64,
    limit: u64,
) -> Vec<Vec<u8>> {
    let size = content.len() as u64;
    let (mut cursor, end) = window(size, start, end, 0);
    let mut skipping = if cursor < end {
        skip + if options.align {
            misaligned(content, cursor, options.boundary)
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
            .position(|byte| *byte == options.boundary)
            .map_or(size, |offset| cursor + offset as u64 + 1);
        let raw = &content[at..stop as usize];

        cursor = stop;

        if skipping > 0 {
            skipping -= 1;

            continue;
        }

        let mut trimmed = raw.strip_suffix(&[options.boundary]).unwrap_or(raw);

        while options.carriage
            && let Some(rest) = trimmed.strip_suffix(b"\r")
        {
            trimmed = rest;
        }

        if options.skip_empty && trimmed.is_empty() {
            continue;
        }

        yielded += 1;
        collected.push(if options.keep {
            raw.to_vec()
        } else {
            trimmed.to_vec()
        });
    }

    collected
}
