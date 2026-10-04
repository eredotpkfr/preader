macro_rules! read_iterator {
    ($builder:expr, $plan:expr) => {{
        let plan = $plan;
        let take = plan.take;
        let mut iterator = plan.apply($builder).build()?;
        let items = match take {
            Some(count) => $crate::common::funcs::try_items(iterator.by_ref().take(count)),
            None => $crate::common::funcs::try_items(iterator.by_ref()),
        };

        items.map(|items| {
            (
                $crate::common::funcs::flatten(items),
                iterator.state().clone(),
            )
        })
    }};
}

pub(crate) use read_iterator;
