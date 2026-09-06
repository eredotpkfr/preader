use crate::LineBuilder;

pub const LINE_BOUNDARY: u8 = b'\n';

impl LineBuilder<'_> {
    pub fn keepends(mut self, keepends: bool) -> Self {
        self.fields.keepends = keepends;
        self
    }

    pub fn skip_empty(mut self, skip_empty: bool) -> Self {
        self.fields.skip_empty = skip_empty;
        self
    }

    pub fn align(mut self, align: bool) -> Self {
        self.fields.align = align;
        self
    }
}
