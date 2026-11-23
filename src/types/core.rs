use std::{fs::File, io::BufReader};

use crate::{
    Byte, Chunk, Error, Line, Segment,
    bases::{builder::PReaderIteratorBuilder, iterator::PReaderIterator},
};

pub type Result<T> = std::result::Result<T, Error>;

pub type FileReader = BufReader<File>;

pub type ByteIterator = PReaderIterator<Byte>;
pub type ChunkIterator = PReaderIterator<Chunk>;
pub type LineIterator = PReaderIterator<Line>;
pub type SegmentIterator = PReaderIterator<Segment>;

pub type ByteBuilder<'a> = PReaderIteratorBuilder<'a, Byte>;
pub type ChunkBuilder<'a> = PReaderIteratorBuilder<'a, Chunk>;
pub type LineBuilder<'a> = PReaderIteratorBuilder<'a, Line>;
pub type SegmentBuilder<'a> = PReaderIteratorBuilder<'a, Segment>;
