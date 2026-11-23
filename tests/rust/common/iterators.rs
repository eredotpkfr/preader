use std::path::Path;

use preader::{
    ByteIterator, ChunkIterator, IteratorOptions, LineIterator, PReader, PReaderIteratorBuilder,
    Result, SegmentIterator, State, StateSource,
};

use crate::common::{
    constants::{TEST_CHUNK_SIZE, TEST_DEFAULT_DELIMITER},
    interfaces::Item,
    macros::iterator::{each, read_iterator},
};

#[derive(Clone, Copy, Debug)]
pub enum IteratorKind {
    Bytes,
    Chunks,
    Lines,
    Segments,
}

impl IteratorKind {
    pub fn open(self, reader: &PReader, file: &Path, plan: Plan) -> Result<AnyIterator> {
        Ok(match self {
            Self::Bytes => AnyIterator::Bytes(plan.apply(reader.bytes(file)).build()?),
            Self::Chunks => AnyIterator::Chunks(plan.apply(reader.chunks(file)).build()?),
            Self::Lines => AnyIterator::Lines(plan.apply(reader.lines(file)).build()?),
            Self::Segments => AnyIterator::Segments(plan.apply(reader.segments(file)).build()?),
        })
    }

    pub fn read(self, reader: &PReader, file: &Path, plan: Plan) -> Result<(Vec<Vec<u8>>, State)> {
        match self {
            Self::Bytes => read_iterator!(reader.bytes(file), plan),
            Self::Chunks => read_iterator!(reader.chunks(file).size(1), plan),
            Self::Lines => read_iterator!(reader.lines(file), plan),
            Self::Segments => {
                read_iterator!(
                    reader.segments(file).delimiter(TEST_DEFAULT_DELIMITER).keep(true),
                    plan
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Reproducer {
    Bytes,
    Chunks,
    Segments,
    Lines,
}

impl Reproducer {
    pub fn open(self, reader: &PReader, path: &Path, plan: Plan) -> AnyIterator {
        match self {
            Self::Bytes => AnyIterator::Bytes(plan.apply(reader.bytes(path)).build().unwrap()),
            Self::Chunks => AnyIterator::Chunks(
                plan.apply(reader.chunks(path).size(TEST_CHUNK_SIZE)).build().unwrap(),
            ),
            Self::Segments => {
                AnyIterator::Segments(plan.apply(reader.segments(path).keep(true)).build().unwrap())
            }
            Self::Lines => {
                AnyIterator::Lines(plan.apply(reader.lines(path).keepends(true)).build().unwrap())
            }
        }
    }
}

#[derive(Debug)]
pub enum AnyIterator {
    Bytes(ByteIterator),
    Chunks(ChunkIterator),
    Lines(LineIterator),
    Segments(SegmentIterator),
}

impl AnyIterator {
    pub fn state(&self) -> &State {
        each!(self, iterator => iterator.state())
    }

    #[cfg(unix)]
    pub fn state_mut(&mut self) -> &mut State {
        each!(self, iterator => iterator.state_mut())
    }

    pub fn percent(&self) -> f64 {
        each!(self, iterator => iterator.percent())
    }

    pub fn save(&mut self) -> Result<&State> {
        each!(self, iterator => iterator.save())
    }
}

impl Iterator for AnyIterator {
    type Item = Result<Vec<u8>>;

    fn next(&mut self) -> Option<Self::Item> {
        each!(self, iterator => iterator.next().map(|item| item.map(Item::bytes)))
    }
}

#[derive(Debug, Default)]
pub struct Plan {
    pub state: StateSource,
    pub options: IteratorOptions,
    pub take: Option<usize>,
}

impl Plan {
    pub fn apply<I>(self, builder: PReaderIteratorBuilder<'_, I>) -> PReaderIteratorBuilder<'_, I> {
        builder.state(self.state).options(self.options)
    }
}
