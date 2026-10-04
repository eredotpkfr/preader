macro_rules! cycle {
    ($cycles:expr, $label:literal, $plain:expr, $cycling:expr) => {{
        let expected = $crate::common::funcs::items($plain.build().unwrap());
        let size = expected.len().div_ceil($cycles - 1);
        let mut rebuilt = Vec::new();

        for _ in 0..$cycles {
            let mut iterator = $cycling.build().unwrap();

            rebuilt.extend($crate::common::funcs::take(&mut iterator, size));
        }

        assert_eq!(rebuilt, expected, $label);
    }};
}

pub(crate) use cycle;
