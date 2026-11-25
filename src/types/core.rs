use std::{fs::File, io::BufReader};

use crate::{
    Byte, Chunk, Error, Line, Segment,
    bases::{builder::PReaderIteratorBuilder, iterator::PReaderIterator},
};

/// Result type of preader's fallible methods, with [`Error`] as the error
pub type Result<T> = std::result::Result<T, Error>;

/// Buffered reader over an open file that every iterator reads through
pub type FileReader = BufReader<File>;

/// Iterator that yields a file one `u8` at a time, built by a [`ByteBuilder`]
pub type ByteIterator = PReaderIterator<Byte>;
/// Iterator that yields a file in `Vec<u8>` chunks of a fixed size, built by a [`ChunkBuilder`]
pub type ChunkIterator = PReaderIterator<Chunk>;
/// Iterator that yields the UTF-8 lines of a file as `String`s, built by a [`LineBuilder`]
pub type LineIterator = PReaderIterator<Line>;
/// Iterator that yields the `Vec<u8>` segments of a file, built by a [`SegmentBuilder`]
pub type SegmentIterator = PReaderIterator<Segment>;

/// Builder of a [`ByteIterator`], returned by [`PReader::bytes`](crate::PReader::bytes)
pub type ByteBuilder<'a> = PReaderIteratorBuilder<'a, Byte>;
/// Builder of a [`ChunkIterator`], returned by [`PReader::chunks`](crate::PReader::chunks)
pub type ChunkBuilder<'a> = PReaderIteratorBuilder<'a, Chunk>;
/// Builder of a [`LineIterator`], returned by [`PReader::lines`](crate::PReader::lines)
pub type LineBuilder<'a> = PReaderIteratorBuilder<'a, Line>;
/// Builder of a [`SegmentIterator`], returned by [`PReader::segments`](crate::PReader::segments)
pub type SegmentBuilder<'a> = PReaderIteratorBuilder<'a, Segment>;
