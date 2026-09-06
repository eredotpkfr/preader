use crate::{Error, Result, Skip, types::window::Window};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IteratorOptions {
    pub start: u64,
    pub end: u64,
    pub skip: u64,
    pub limit: u64,
}

impl Default for IteratorOptions {
    fn default() -> Self {
        Self {
            start: 0,
            end: u64::MAX,
            skip: 0,
            limit: u64::MAX,
        }
    }
}

impl IteratorOptions {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.start > self.end {
            return Err(Error::Bounds {
                start: self.start,
                end: self.end,
            });
        }

        Ok(())
    }

    pub fn window(&self, position: u64, size: u64, skip: Skip) -> Window {
        let end = self.end.min(size);
        let start = self.start.saturating_add(skip.bytes()).min(end);

        Window {
            position: position.max(start),
            end,
            skipping: (position <= start && position < end).then(|| skip.items()),
        }
    }
}
