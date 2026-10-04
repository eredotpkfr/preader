use std::path::Path;

use preader::{
    IteratorBuild, IteratorOptions, PReader, PReaderIteratorBuilder, Result, State, StateSource,
};

use crate::common::{constants::TEST_DELIMITER, macros::kind::read_kind};

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Byte,
    Chunk,
    Delimiter,
    Line,
}

pub const KINDS: [Kind; 4] = [Kind::Byte, Kind::Chunk, Kind::Delimiter, Kind::Line];
pub const LOSSLESS_KINDS: [Kind; 3] = [Kind::Byte, Kind::Chunk, Kind::Delimiter];

#[derive(Debug, Default)]
pub struct Setup {
    pub state: StateSource,
    pub options: IteratorOptions,
    pub take: Option<usize>,
}

impl Setup {
    pub fn stateful(state: State) -> Self {
        Self {
            state: state.into(),
            ..Self::default()
        }
    }

    pub fn apply<I>(self, builder: PReaderIteratorBuilder<'_, I>) -> PReaderIteratorBuilder<'_, I> {
        builder.state(self.state).options(self.options)
    }
}

impl Kind {
    pub fn read(
        self,
        reader: &PReader,
        file: &Path,
        setup: Setup,
    ) -> Result<(Vec<Vec<u8>>, State)> {
        match self {
            Self::Byte => read_kind!(reader.bytes(file), setup),
            Self::Chunk => read_kind!(reader.chunks(file).size(1), setup),
            Self::Delimiter => {
                read_kind!(
                    reader.delimiter(file).character(TEST_DELIMITER).keep(true),
                    setup
                )
            }
            Self::Line => read_kind!(reader.lines(file), setup),
        }
    }
}
