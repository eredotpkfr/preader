use crate::enums::skip::SkipPlan;

pub trait Skip {
    fn skip(&self, count: u64) -> SkipPlan;
}
