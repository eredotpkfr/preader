#[derive(Clone, Copy, Debug)]
pub enum Skip {
    Bytes(u64),
    Items { count: u64, boundary: Option<u8> },
}

impl Skip {
    pub(crate) fn counts(self) -> (u64, u64) {
        match self {
            Self::Bytes(bytes) => (bytes, 0),
            Self::Items { count, .. } => (0, count),
        }
    }

    pub(crate) fn boundary(self) -> Option<u8> {
        match self {
            Self::Bytes(_) => None,
            Self::Items { boundary, .. } => boundary,
        }
    }
}
