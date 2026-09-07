use std::io::Read;

use crate::{
    ByteIterator, Result, Skip,
    interfaces::{iterator::IteratorRead, skippable::Skippable},
};

#[derive(Debug, Default)]
pub struct Byte;

impl IteratorRead for ByteIterator {
    type Borrowed<'a> = u8;
    type Owned = u8;

    fn read(&mut self) -> Result<Option<u8>> {
        if self.done() {
            return self.stop();
        }

        let Some(byte) = (&mut self.reader).bytes().next().transpose()? else {
            return self.stop();
        };

        self.progress.count();
        self.advance(1)?;

        Ok(Some(byte))
    }
}

impl Skippable for Byte {
    fn skip(&self, count: u64) -> Skip {
        Skip::Bytes(count)
    }
}
