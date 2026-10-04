macro_rules! drain {
    ($builder:expr, $size:expr) => {{
        let mut iterator = $builder.build().unwrap();

        while iterator.read().unwrap().is_some() {
            assert!(iterator.state().position <= $size);
        }

        assert_eq!(iterator.state().position, $size);
        assert_eq!(iterator.state().percent(), 100.0);
    }};
}

pub(crate) use drain;
