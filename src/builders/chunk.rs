use crate::ChunkBuilder;

impl ChunkBuilder<'_> {
    pub fn size(mut self, size: usize) -> Self {
        self.fields.size = size;
        self.fields.buffer.resize(size, 0);
        self
    }

    pub fn drop_partial(mut self, drop_partial: bool) -> Self {
        self.fields.drop_partial = drop_partial;
        self
    }
}
