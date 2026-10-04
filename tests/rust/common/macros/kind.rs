macro_rules! read_kind {
    ($builder:expr, $setup:expr) => {{
        let setup = $setup;
        let take = setup.take;
        let mut iterator = setup.apply($builder).build()?;
        let items = match take {
            Some(count) => $crate::common::funcs::try_items(iterator.by_ref().take(count)),
            None => $crate::common::funcs::try_items(iterator.by_ref()),
        };

        items.map(|items| (items, iterator.state().clone()))
    }};
}

pub(crate) use read_kind;
