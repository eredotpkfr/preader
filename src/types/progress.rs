use std::mem;

/// Bookkeeping of one read: its end, its item budget and the item still unfinished
///
/// The unfinished item's bytes stay in [`pending`] until the next item is requested, which is
/// when [`State::position`](crate::State::position) moves past them
///
/// [`pending`]: Self::pending
#[derive(Debug)]
pub(crate) struct Progress {
    /// Byte offset at which no new item may start
    pub(crate) end: u64,
    /// Maximum number of items to yield
    pub(crate) limit: u64,
    /// Number of items yielded so far
    pub(crate) yielded: u64,
    /// Number of items still to read and drop before yielding
    pub(crate) skipping: u64,
    /// Bytes of the last item read, not yet added to the position
    pub(crate) pending: u64,
}

impl Progress {
    /// Returns whether the read has reached [`end`] or yielded [`limit`] items
    ///
    /// [`end`]: Self::end
    /// [`limit`]: Self::limit
    pub(crate) fn done(&self, position: u64) -> bool {
        position + self.pending >= self.end || self.yielded >= self.limit
    }

    /// Returns the bytes left before [`end`](Self::end), counting the pending item as read
    pub(crate) fn remaining(&self, position: u64) -> u64 {
        self.end.saturating_sub(position + self.pending)
    }

    /// Consumes one item of the skip budget and returns whether the current item is skipped
    pub(crate) fn skip(&mut self) -> bool {
        let skipping = self.skipping > 0;

        self.skipping = self.skipping.saturating_sub(1);

        skipping
    }

    /// Records `bytes` as the pending item
    pub(crate) fn hold(&mut self, bytes: u64) {
        self.pending = bytes;
    }

    /// Returns the pending bytes and resets them to zero
    pub(crate) fn take(&mut self) -> u64 {
        mem::take(&mut self.pending)
    }

    /// Counts one more yielded item
    pub(crate) fn tick(&mut self) {
        self.yielded += 1;
    }

    /// Lowers the limit to the items yielded so far, so the read is done
    pub(crate) fn exhaust(&mut self) {
        self.limit = self.yielded;
    }
}
