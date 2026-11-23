use crate::SegmentBuilder;

impl SegmentBuilder<'_> {
    pub fn delimiter(mut self, delimiter: u8) -> Self {
        self.inner.delimiter = delimiter;
        self
    }

    pub fn keep(mut self, keep: bool) -> Self {
        self.inner.keep = keep;
        self
    }

    pub fn skip_empty(mut self, skip_empty: bool) -> Self {
        self.inner.skip_empty = skip_empty;
        self
    }

    pub fn align(mut self, align: bool) -> Self {
        self.inner.align = align;
        self
    }
}
