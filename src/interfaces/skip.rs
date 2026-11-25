use crate::enums::skip::SkipPlan;

/// Strategy-specific meaning of [`IteratorOptions::skip`](crate::IteratorOptions::skip)
pub trait Skip {
    /// Returns the plan that skips `count` items of this strategy
    fn skip(&self, count: u64) -> SkipPlan;
}
