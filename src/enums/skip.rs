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
