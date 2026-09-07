use crate::LineBuilder;

impl LineBuilder<'_> {
    pub fn keepends(mut self, keepends: bool) -> Self {
        self.inner.keepends = keepends;
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
