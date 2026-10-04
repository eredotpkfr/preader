use std::path::Path;

use preader::{
    DEFAULT_DELIMITER, IteratorBuild, IteratorOptions, PReader, PReaderIteratorBuilder, Result,
    State, StateSource,
};

use crate::common::macros::iterator::read_iterator;

#[derive(Clone, Copy, Debug)]
pub enum IteratorKind {
    Bytes,
    Chunks,
    Delimiter,
    Lines,
}

pub const ITERATORS: [IteratorKind; 4] = [
    IteratorKind::Bytes,
    IteratorKind::Chunks,
    IteratorKind::Delimiter,
    IteratorKind::Lines,
];
pub const LOSSLESS_ITERATORS: [IteratorKind; 3] = [
    IteratorKind::Bytes,
    IteratorKind::Chunks,
    IteratorKind::Delimiter,
];

#[derive(Debug, Default)]
pub struct Plan {
    pub options: IteratorOptions,
    pub state: StateSource,
    pub take: Option<usize>,
}

impl Plan {
    pub fn resuming(state: State) -> Self {
        Self {
            state: state.into(),
            ..Self::default()
        }
    }

    pub fn apply<I>(self, builder: PReaderIteratorBuilder<'_, I>) -> PReaderIteratorBuilder<'_, I> {
        builder.state(self.state).options(self.options)
    }
}

impl IteratorKind {
    pub fn read(self, reader: &PReader, file: &Path, plan: Plan) -> Result<(Vec<Vec<u8>>, State)> {
        match self {
            Self::Bytes => read_iterator!(reader.bytes(file), plan),
            Self::Chunks => read_iterator!(reader.chunks(file).size(1), plan),
            Self::Delimiter => {
                read_iterator!(
                    reader.delimiter(file).character(DEFAULT_DELIMITER).keep(true),
                    plan
                )
            }
            Self::Lines => read_iterator!(reader.lines(file), plan),
        }
    }
}
