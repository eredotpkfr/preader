use crate::Result;

pub trait IteratorRead {
    type Borrowed<'a>
    where
        Self: 'a;
    type Owned: for<'a> From<Self::Borrowed<'a>>;

    fn read(&mut self) -> Result<Option<Self::Borrowed<'_>>>;
}
