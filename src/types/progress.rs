#[derive(Debug)]
pub struct Progress {
    pub(crate) end: u64,
    pub(crate) limit: u64,
    pub(crate) yielded: u64,
    pub(crate) skipping: u64,
}

impl Progress {
    pub(crate) fn new(end: u64, limit: u64, skipping: u64) -> Self {
        Self {
            end,
            limit,
            yielded: 0,
            skipping,
        }
    }

    pub(crate) fn done(&self, position: u64) -> bool {
        position >= self.end || self.yielded >= self.limit
    }

    pub(crate) fn remaining(&self, position: u64) -> u64 {
        self.end.saturating_sub(position)
    }

    pub(crate) fn skip(&mut self) -> bool {
        self.skipping = match self.skipping.checked_sub(1) {
            Some(remaining) => remaining,
            None => return false,
        };

        true
    }

    pub(crate) fn count(&mut self) {
        self.yielded += 1;
    }
}
