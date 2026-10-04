use preader::IteratorOptions;

use crate::common::matrix::expected::cursor::cursor;

pub fn bytes(content: &[u8], options: IteratorOptions) -> Vec<u8> {
    let size = content.len() as u64;
    let (from, end) = cursor(size, options, options.skip);
    let to = end.min(from.saturating_add(options.limit)).min(size);

    content[(from.min(size) as usize)..(to.max(from).min(size) as usize)].to_vec()
}
