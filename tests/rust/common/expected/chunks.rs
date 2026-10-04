use preader::IteratorOptions;

use crate::common::expected::cursor::cursor;

pub fn chunks(
    content: &[u8],
    size_of: usize,
    options: IteratorOptions,
    drop_partial: bool,
) -> Vec<Vec<u8>> {
    let size = content.len() as u64;
    let stride = options.skip.saturating_mul(size_of as u64);
    let (mut at, end) = cursor(size, options, stride);
    let (mut yielded, mut collected) = (0, Vec::new());

    while at < end && yielded < options.limit {
        let max = (end - at).min(size_of as u64);
        let filled = max.min(size.saturating_sub(at));

        if filled == 0 || (drop_partial && filled < size_of as u64) {
            break;
        }

        collected.push(content[at as usize..(at + filled) as usize].to_vec());
        at += filled;
        yielded += 1;
    }

    collected
}
