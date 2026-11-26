use std::io::{Cursor, Write};

use pdf_writer::{Pdf, Rect, Ref, Str};
use shake::{ExtendableOutput, Shake256, Update, XofReader};
use zip::{CompressionMethod, DateTime, ZipWriter, write::SimpleFileOptions};

use crate::common::{constants::TEST_UNICODE_TEXT, seed::Seed};

const MAX_WORDS_PER_LINE: usize = 12;
const LINE_LENGTH: usize = 40;
const MEMBER_COUNT: u64 = 3;
const ATTACHMENT_NAME: &[u8] = b"seeded.bin";

#[derive(Clone, Copy, Debug)]
pub enum Kind {
    Random,
    Text,
    Zip,
    Pdf,
}

pub fn seeded_content(kind: Kind, size: usize, Seed(seed): Seed) -> Vec<u8> {
    match kind {
        Kind::Random => seeded_bytes(seed, size),
        Kind::Text => seeded_text(seed, size).into_bytes(),
        Kind::Zip => seeded_zip(seed, size),
        Kind::Pdf => seeded_pdf(seed, size),
    }
}

fn seeded_bytes(seed: u64, size: usize) -> Vec<u8> {
    let mut shake = Shake256::default();
    let mut output = vec![0; size];

    shake.update(seed.to_string().as_bytes());
    shake.finalize_xof().read(&mut output);

    output
}

fn seeded_text(seed: u64, size: usize) -> String {
    let words: Vec<&str> = ["preader", "iterator", "state", "foo", "bar", "baz"]
        .into_iter()
        .chain(TEST_UNICODE_TEXT.split_whitespace())
        .chain(["$qux", "`quux`", "a;b", ","])
        .collect();

    let line_count = size / LINE_LENGTH;
    let mut stream = seeded_bytes(seed, line_count * (MAX_WORDS_PER_LINE + 1)).into_iter();
    let mut next = || usize::from(stream.next().unwrap());

    (0..line_count)
        .map(|_| {
            let count = next() % (MAX_WORDS_PER_LINE + 1);
            let line: Vec<&str> = (0..count).map(|_| words[next() % words.len()]).collect();

            format!("{}\n", line.join(" "))
        })
        .collect()
}

fn seeded_zip(seed: u64, size: usize) -> Vec<u8> {
    let mut archive = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .last_modified_time(DateTime::default());

    for offset in 0..MEMBER_COUNT {
        archive.start_file(format!("member-{offset}.bin"), options).unwrap();
        archive
            .write_all(&seeded_bytes(
                seed.wrapping_add(offset),
                size / MEMBER_COUNT as usize,
            ))
            .unwrap();
    }

    archive.finish().unwrap().into_inner()
}

fn seeded_pdf(seed: u64, size: usize) -> Vec<u8> {
    let attachment = seeded_bytes(seed, size);
    let [catalog, pages, page, spec, file] = [1, 2, 3, 4, 5].map(Ref::new);
    let mut pdf = Pdf::new();

    pdf.catalog(catalog)
        .pages(pages)
        .names()
        .embedded_files()
        .names()
        .insert(Str(ATTACHMENT_NAME), spec);
    pdf.pages(pages).kids([page]).count(1);
    pdf.page(page).parent(pages).media_box(Rect::new(0.0, 0.0, 612.0, 792.0));
    pdf.file_spec(spec).path(Str(ATTACHMENT_NAME)).embedded_file(file);
    pdf.embedded_file(file, &attachment);

    pdf.finish()
}
