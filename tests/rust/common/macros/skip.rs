macro_rules! skip {
    ($reason:literal) => {{
        eprintln!("skipped: {}", $reason);

        return;
    }};
}

pub(crate) use skip;
