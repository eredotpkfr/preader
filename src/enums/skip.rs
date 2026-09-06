use crate::{
    ByteBuilder, ChunkBuilder, DelimiterBuilder, LineBuilder, builders::line::LINE_BOUNDARY,
};

#[derive(Clone, Copy, Debug)]
pub enum Skip {
    Bytes(u64),
    Items { count: u64, boundary: Option<u8> },
}

impl Skip {
    pub(crate) fn bytes(self) -> u64 {
        match self {
            Self::Bytes(bytes) => bytes,
            Self::Items { .. } => 0,
        }
    }

    pub(crate) fn items(self) -> u64 {
        match self {
            Self::Bytes(_) => 0,
            Self::Items { count, .. } => count,
        }
    }

    pub(crate) fn boundary(self) -> Option<u8> {
        match self {
            Self::Bytes(_) => None,
            Self::Items { boundary, .. } => boundary,
        }
    }
}
impl From<&ByteBuilder<'_>> for Skip {
    fn from(builder: &ByteBuilder<'_>) -> Self {
        Self::Bytes(builder.options.skip)
    }
}

impl From<&ChunkBuilder<'_>> for Skip {
    fn from(builder: &ChunkBuilder<'_>) -> Self {
        Self::Bytes(builder.options.skip.saturating_mul(builder.fields.size as u64))
    }
}

impl From<&LineBuilder<'_>> for Skip {
    fn from(builder: &LineBuilder<'_>) -> Self {
        Self::Items {
            count: builder.options.skip,
            boundary: builder.fields.align.then_some(LINE_BOUNDARY),
        }
    }
}

impl From<&DelimiterBuilder<'_>> for Skip {
    fn from(builder: &DelimiterBuilder<'_>) -> Self {
        Self::Items {
            count: builder.options.skip,
            boundary: builder.fields.align.then_some(builder.fields.character),
        }
    }
}
