#[derive(Debug)]
pub struct Progress {
    pub(crate) end: u64,
    pub(crate) limit: u64,
    pub(crate) yielded: u64,
    pub(crate) skipping: u64,
}

impl Progress {
    pub(crate) fn done(&self, position: u64) -> bool {
        position >= self.end || self.yielded >= self.limit
    }

    pub(crate) fn remaining(&self, position: u64) -> u64 {
        self.end.saturating_sub(position)
    }

    pub(crate) fn skip(&mut self) -> bool {
        let skipping = self.skipping > 0;

        self.skipping -= u64::from(skipping);

        skipping
    }

    pub(crate) fn count(&mut self) {
        self.yielded += 1;
    }

    pub(crate) fn exhaust(&mut self) {
        self.limit = self.yielded;
    }
}
