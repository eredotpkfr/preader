use crate::enums::skip::Skip;

pub trait Skippable {
    fn skip(&self, count: u64) -> Skip;
}
