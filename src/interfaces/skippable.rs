use crate::Skip;

pub trait Skippable {
    fn skip(&self, count: u64) -> Skip;
}
