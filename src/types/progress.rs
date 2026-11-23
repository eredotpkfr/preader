use std::mem;

#[derive(Debug)]
pub(crate) struct Progress {
    pub(crate) end: u64,
    pub(crate) limit: u64,
    pub(crate) yielded: u64,
    pub(crate) skipping: u64,
    pub(crate) pending: u64,
}

impl Progress {
    pub(crate) fn done(&self, position: u64) -> bool {
        position + self.pending >= self.end || self.yielded >= self.limit
    }

    pub(crate) fn remaining(&self, position: u64) -> u64 {
        self.end.saturating_sub(position + self.pending)
    }

    pub(crate) fn skip(&mut self) -> bool {
        let skipping = self.skipping > 0;

        self.skipping = self.skipping.saturating_sub(1);

        skipping
    }

    pub(crate) fn hold(&mut self, bytes: u64) {
        self.pending = bytes;
    }

    pub(crate) fn take(&mut self) -> u64 {
        mem::take(&mut self.pending)
    }

    pub(crate) fn tick(&mut self) {
        self.yielded += 1;
    }

    pub(crate) fn exhaust(&mut self) {
        self.limit = self.yielded;
    }
}
