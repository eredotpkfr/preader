use std::io::Read;

use crate::{
    ByteIterator, Result,
    enums::skip::SkipPlan,
    interfaces::{iterator::IteratorRead, skip::Skip},
};

#[derive(Debug, Default)]
pub struct Byte;

impl IteratorRead for ByteIterator {
    type Borrowed<'a> = u8;
    type Owned = u8;

    fn read(&mut self) -> Result<Option<u8>> {
        if self.done() {
            return self.finalize();
        }

        let (read, byte) = match (&mut self.reader).bytes().next().transpose() {
            Ok(byte) => (Ok(usize::from(byte.is_some())), byte),
            Err(error) => (Err(error.into()), None),
        };

        if self.advance(read)? == 0 {
            return self.finalize();
        }

        self.progress.tick();

        Ok(byte)
    }
}

impl Skip for Byte {
    fn skip(&self, count: u64) -> SkipPlan {
        SkipPlan::Bytes(count)
    }
}
