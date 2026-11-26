use preader::IteratorOptions;

const LINE_BOUNDARY: u8 = b'\n';

#[derive(Clone, Copy, Debug)]
pub struct Shape {
    pub keep: bool,
    pub skip_empty: bool,
    pub align: bool,
}

pub fn bytes(content: &[u8], options: IteratorOptions) -> Vec<u8> {
    let (start, end) = window(content, options);

    content[start..end]
        .iter()
        .copied()
        .skip(count(options.skip))
        .take(count(options.limit))
        .collect()
}

pub fn chunks(
    content: &[u8],
    size: usize,
    options: IteratorOptions,
    drop_partial: bool,
) -> Vec<Vec<u8>> {
    let (start, end) = window(content, options);

    if size == 0 {
        return Vec::new();
    }

    content[start..end]
        .chunks(size)
        .filter(|chunk| !drop_partial || chunk.len() == size)
        .skip(count(options.skip))
        .take(count(options.limit))
        .map(<[u8]>::to_vec)
        .collect()
}

pub fn lines(content: &[u8], shape: Shape, options: IteratorOptions) -> Option<Vec<String>> {
    split(content, LINE_BOUNDARY, true, shape, options)
        .into_iter()
        .map(|line| String::from_utf8(line).ok())
        .collect()
}

pub fn segments(
    content: &[u8],
    shape: Shape,
    options: IteratorOptions,
    boundary: u8,
) -> Vec<Vec<u8>> {
    split(content, boundary, false, shape, options)
}

fn split(
    content: &[u8],
    boundary: u8,
    carriage: bool,
    shape: Shape,
    options: IteratorOptions,
) -> Vec<Vec<u8>> {
    let (start, end) = window(content, options);
    let misaligned = shape.align && start > 0 && content[start - 1] != boundary;
    let mut offset = start;

    content[start..]
        .split_inclusive(|byte| *byte == boundary)
        .take_while(|raw| {
            let from = offset;
            offset += raw.len();

            from < end
        })
        .skip(count(options.skip).saturating_add(usize::from(misaligned)))
        .map(|raw| (raw, trim(raw, boundary, carriage)))
        .filter(|(_, trimmed)| !(shape.skip_empty && trimmed.is_empty()))
        .take(count(options.limit))
        .map(|(raw, trimmed)| if shape.keep { raw } else { trimmed }.to_vec())
        .collect()
}

fn trim(raw: &[u8], boundary: u8, carriage: bool) -> &[u8] {
    let mut trimmed = raw.strip_suffix(&[boundary]).unwrap_or(raw);

    while carriage && let Some(rest) = trimmed.strip_suffix(b"\r") {
        trimmed = rest;
    }

    trimmed
}

fn window(content: &[u8], options: IteratorOptions) -> (usize, usize) {
    let end = options.end.min(content.len() as u64);

    (options.start.min(end) as usize, end as usize)
}

fn count(items: u64) -> usize {
    usize::try_from(items).unwrap_or(usize::MAX)
}
