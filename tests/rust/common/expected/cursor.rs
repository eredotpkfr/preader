use preader::IteratorOptions;

pub fn cursor(size: u64, options: IteratorOptions, skipped: u64) -> (u64, u64) {
    let end = options.end.min(size);

    (options.start.saturating_add(skipped).min(end), end)
}
