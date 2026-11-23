use std::{fmt, ops::Deref};

use preader::{DEFAULT_BUFFER_CAPACITY, IteratorOptions};
use proptest::{collection::vec, prelude::*, sample::select};

use crate::common::matrix::contents::{
    ALL_BYTES, BLANK_LINES, BLANK_SEGMENTS, CRLF_LINES, EMPTY, LINES, MISALIGNED_SEGMENTS,
    SEGMENTS, UNTERMINATED_LINES, UNTERMINATED_SEGMENTS, UTF8_SEGMENTS, WHOLE_SEGMENT,
};

const HANDWRITTEN: [&[u8]; 12] = [
    EMPTY,
    LINES,
    UNTERMINATED_LINES,
    CRLF_LINES,
    BLANK_LINES,
    SEGMENTS,
    UNTERMINATED_SEGMENTS,
    BLANK_SEGMENTS,
    WHOLE_SEGMENT,
    MISALIGNED_SEGMENTS,
    UTF8_SEGMENTS,
    ALL_BYTES,
];
const DENSE: &[u8] = b",\n\r\0x\x7f\x80\xe9\xff";
const LETTERS: [char; 6] = ['a', ',', '\n', '\r', 'é', '🦀'];
const MAX_BYTES: usize = 64;
const MAX_LETTERS: usize = 16;
const MAX_CAPACITY: usize = 16;

#[derive(Clone)]
pub struct Content(Vec<u8>);

impl Deref for Content {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for Content {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "b\"{}\"", self.0.escape_ascii())
    }
}

pub fn bytes() -> impl Strategy<Value = (Content, IteratorOptions, usize)> {
    contents().prop_flat_map(|content| {
        let size = content.len() as u64;

        (Just(content), options(size, size), capacities())
    })
}

pub fn chunks() -> impl Strategy<Value = (Content, IteratorOptions, usize, usize)> {
    contents()
        .prop_flat_map(|content| {
            let size = content.len();

            (Just(content), 0..=size + 2)
        })
        .prop_flat_map(|(content, chunk)| {
            let size = content.len();
            let items = size.checked_div(chunk).map_or(0, |whole| whole + 1);

            (
                Just(content),
                options(size as u64, items as u64),
                Just(chunk),
                capacities(),
            )
        })
}

pub fn lines() -> impl Strategy<Value = (Content, IteratorOptions, usize)> {
    prop_oneof![handwritten(), text()].prop_flat_map(|content| delimited(content, b'\n'))
}

pub fn segments(delimiter: u8) -> impl Strategy<Value = (Content, IteratorOptions, usize)> {
    contents().prop_flat_map(move |content| delimited(content, delimiter))
}

fn delimited(
    content: Content,
    delimiter: u8,
) -> impl Strategy<Value = (Content, IteratorOptions, usize)> {
    let size = content.len() as u64;
    let items = content.split(|byte| *byte == delimiter).count() as u64;

    (Just(content), options(size, items), capacities())
}

fn contents() -> impl Strategy<Value = Content> {
    prop_oneof![handwritten(), binary(), text()]
}

fn handwritten() -> impl Strategy<Value = Content> {
    select(HANDWRITTEN.as_slice()).prop_map(|content| Content(content.to_vec()))
}

fn binary() -> impl Strategy<Value = Content> {
    vec(prop_oneof![select(DENSE), any::<u8>()], 0..=MAX_BYTES).prop_map(Content)
}

fn text() -> impl Strategy<Value = Content> {
    vec(select(LETTERS.as_slice()), 0..=MAX_LETTERS)
        .prop_map(|letters| Content(String::from_iter(letters).into_bytes()))
}

fn capacities() -> impl Strategy<Value = usize> {
    prop_oneof![1..=MAX_CAPACITY, Just(DEFAULT_BUFFER_CAPACITY)]
}

fn options(size: u64, items: u64) -> impl Strategy<Value = IteratorOptions> {
    (bounds(size), 0..=items + 1, position(items)).prop_map(|((start, end), skip, limit)| {
        IteratorOptions {
            start,
            end,
            skip,
            limit,
        }
    })
}

fn bounds(size: u64) -> impl Strategy<Value = (u64, u64)> {
    let ordered = (position(size), position(size)).prop_map(|(one, other)| sorted(one, other));
    let inverted = (0..=size, 0..=size).prop_map(|(one, other)| {
        let (low, high) = sorted(one, other);

        (high + 1, low)
    });

    prop_oneof![9 => ordered, 1 => inverted]
}

fn position(size: u64) -> impl Strategy<Value = u64> {
    prop_oneof![9 => 0..=size + 1, 1 => Just(u64::MAX)]
}

fn sorted(one: u64, other: u64) -> (u64, u64) {
    (one.min(other), one.max(other))
}
