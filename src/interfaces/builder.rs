use crate::{Result, interfaces::iterator::IteratorRead};

pub trait IteratorBuild {
    type Iterator: IteratorRead;

    fn build(self) -> Result<Self::Iterator>;
}
