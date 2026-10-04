use std::path::Path;

use preader::{
    DEFAULT_DELIMITER, IteratorBuild, IteratorOptions, PReader, PReaderIteratorBuilder, Result,
    State, StateSource,
};

use crate::common::macros::flow::read_flow;

#[derive(Clone, Copy, Debug)]
pub enum Flow {
    Bytes,
    Chunks,
    Delimiter,
    Lines,
}

pub const FLOWS: [Flow; 4] = [Flow::Bytes, Flow::Chunks, Flow::Delimiter, Flow::Lines];
pub const LOSSLESS_FLOWS: [Flow; 3] = [Flow::Bytes, Flow::Chunks, Flow::Delimiter];

#[derive(Debug, Default)]
pub struct Plan {
    pub state: StateSource,
    pub options: IteratorOptions,
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

impl Flow {
    pub fn read(self, reader: &PReader, file: &Path, plan: Plan) -> Result<(Vec<Vec<u8>>, State)> {
        match self {
            Self::Bytes => read_flow!(reader.bytes(file), plan),
            Self::Chunks => read_flow!(reader.chunks(file).size(1), plan),
            Self::Delimiter => {
                read_flow!(
                    reader.delimiter(file).character(DEFAULT_DELIMITER).keep(true),
                    plan
                )
            }
            Self::Lines => read_flow!(reader.lines(file), plan),
        }
    }
}
