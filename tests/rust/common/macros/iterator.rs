macro_rules! each {
    ($iterator:expr, $inner:ident => $body:expr) => {
        match $iterator {
            $crate::common::iterators::AnyIterator::Bytes($inner) => $body,
            $crate::common::iterators::AnyIterator::Chunks($inner) => $body,
            $crate::common::iterators::AnyIterator::Lines($inner) => $body,
            $crate::common::iterators::AnyIterator::Segments($inner) => $body,
        }
    };
}

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

pub(crate) use each;
pub(crate) use read_iterator;
