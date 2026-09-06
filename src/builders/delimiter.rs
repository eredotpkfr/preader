use crate::DelimiterBuilder;

impl DelimiterBuilder<'_> {
    pub fn character(mut self, character: u8) -> Self {
        self.fields.character = character;
        self
    }

    pub fn keep(mut self, keep: bool) -> Self {
        self.fields.keep = keep;
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
