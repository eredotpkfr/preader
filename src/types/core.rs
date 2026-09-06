use std::{fs::File, io::BufReader};

use crate::{
    Byte, Chunk, Delimiter, Error, Line,
    bases::{builder::PReaderIteratorBuilder, iterator::PReaderIterator},
};

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) type Reader = BufReader<File>;

pub type ByteIterator = PReaderIterator<Byte>;
pub type ChunkIterator = PReaderIterator<Chunk>;
pub type DelimiterIterator = PReaderIterator<Delimiter>;
pub type LineIterator = PReaderIterator<Line>;

pub type ByteBuilder<'a> = PReaderIteratorBuilder<'a, Byte>;
pub type ChunkBuilder<'a> = PReaderIteratorBuilder<'a, Chunk>;
pub type DelimiterBuilder<'a> = PReaderIteratorBuilder<'a, Delimiter>;
pub type LineBuilder<'a> = PReaderIteratorBuilder<'a, Line>;
