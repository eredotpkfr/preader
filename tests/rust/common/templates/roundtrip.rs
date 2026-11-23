use rstest_reuse::template;

#[template]
#[rstest]
#[case::random_512b(crate::common::rng::Kind::Random, 512)]
#[case::random_4096b(crate::common::rng::Kind::Random, 4_096)]
#[case::random_51200b(crate::common::rng::Kind::Random, 51_200)]
#[case::text_512b(crate::common::rng::Kind::Text, 512)]
#[case::text_4096b(crate::common::rng::Kind::Text, 4_096)]
#[case::text_51200b(crate::common::rng::Kind::Text, 51_200)]
#[case::zip_512b(crate::common::rng::Kind::Zip, 512)]
#[case::zip_4096b(crate::common::rng::Kind::Zip, 4_096)]
#[case::zip_51200b(crate::common::rng::Kind::Zip, 51_200)]
#[case::pdf_512b(crate::common::rng::Kind::Pdf, 512)]
#[case::pdf_4096b(crate::common::rng::Kind::Pdf, 4_096)]
#[case::pdf_51200b(crate::common::rng::Kind::Pdf, 51_200)]
fn contents(#[case] kind: crate::common::rng::Kind, #[case] size: usize) {}

#[template]
#[rstest]
#[case::text_512b(crate::common::rng::Kind::Text, 512)]
#[case::text_4096b(crate::common::rng::Kind::Text, 4_096)]
#[case::text_51200b(crate::common::rng::Kind::Text, 51_200)]
fn texts(#[case] kind: crate::common::rng::Kind, #[case] size: usize) {}

#[template]
#[rstest]
#[case::zip_512b(crate::common::rng::Kind::Zip, 512)]
#[case::zip_4096b(crate::common::rng::Kind::Zip, 4_096)]
#[case::zip_51200b(crate::common::rng::Kind::Zip, 51_200)]
fn archives(#[case] kind: crate::common::rng::Kind, #[case] size: usize) {}

#[template]
#[rstest]
#[case::bytes_random_512b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Random,
    512
)]
#[case::bytes_random_4096b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Random,
    4_096
)]
#[case::bytes_random_51200b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Random,
    51_200
)]
#[case::bytes_text_512b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Text,
    512
)]
#[case::bytes_text_4096b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Text,
    4_096
)]
#[case::bytes_text_51200b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Text,
    51_200
)]
#[case::bytes_zip_512b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Zip,
    512
)]
#[case::bytes_zip_4096b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Zip,
    4_096
)]
#[case::bytes_zip_51200b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Zip,
    51_200
)]
#[case::bytes_pdf_512b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Pdf,
    512
)]
#[case::bytes_pdf_4096b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Pdf,
    4_096
)]
#[case::bytes_pdf_51200b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Pdf,
    51_200
)]
#[case::chunks_random_512b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Random,
    512
)]
#[case::chunks_random_4096b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Random,
    4_096
)]
#[case::chunks_random_51200b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Random,
    51_200
)]
#[case::chunks_text_512b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Text,
    512
)]
#[case::chunks_text_4096b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Text,
    4_096
)]
#[case::chunks_text_51200b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Text,
    51_200
)]
#[case::chunks_zip_512b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Zip,
    512
)]
#[case::chunks_zip_4096b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Zip,
    4_096
)]
#[case::chunks_zip_51200b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Zip,
    51_200
)]
#[case::chunks_pdf_512b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Pdf,
    512
)]
#[case::chunks_pdf_4096b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Pdf,
    4_096
)]
#[case::chunks_pdf_51200b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Pdf,
    51_200
)]
fn windows(
    #[case] reproducer: crate::common::iterators::Reproducer,
    #[case] kind: crate::common::rng::Kind,
    #[case] size: usize,
) {
}

#[template]
#[rstest]
#[case::bytes_random_512b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Random,
    512
)]
#[case::bytes_random_4096b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Random,
    4_096
)]
#[case::bytes_random_51200b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Random,
    51_200
)]
#[case::bytes_text_512b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Text,
    512
)]
#[case::bytes_text_4096b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Text,
    4_096
)]
#[case::bytes_text_51200b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Text,
    51_200
)]
#[case::bytes_zip_512b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Zip,
    512
)]
#[case::bytes_zip_4096b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Zip,
    4_096
)]
#[case::bytes_zip_51200b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Zip,
    51_200
)]
#[case::bytes_pdf_512b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Pdf,
    512
)]
#[case::bytes_pdf_4096b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Pdf,
    4_096
)]
#[case::bytes_pdf_51200b(
    crate::common::iterators::Reproducer::Bytes,
    crate::common::rng::Kind::Pdf,
    51_200
)]
#[case::chunks_random_512b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Random,
    512
)]
#[case::chunks_random_4096b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Random,
    4_096
)]
#[case::chunks_random_51200b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Random,
    51_200
)]
#[case::chunks_text_512b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Text,
    512
)]
#[case::chunks_text_4096b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Text,
    4_096
)]
#[case::chunks_text_51200b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Text,
    51_200
)]
#[case::chunks_zip_512b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Zip,
    512
)]
#[case::chunks_zip_4096b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Zip,
    4_096
)]
#[case::chunks_zip_51200b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Zip,
    51_200
)]
#[case::chunks_pdf_512b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Pdf,
    512
)]
#[case::chunks_pdf_4096b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Pdf,
    4_096
)]
#[case::chunks_pdf_51200b(
    crate::common::iterators::Reproducer::Chunks,
    crate::common::rng::Kind::Pdf,
    51_200
)]
#[case::segments_random_512b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Random,
    512
)]
#[case::segments_random_4096b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Random,
    4_096
)]
#[case::segments_random_51200b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Random,
    51_200
)]
#[case::segments_text_512b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Text,
    512
)]
#[case::segments_text_4096b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Text,
    4_096
)]
#[case::segments_text_51200b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Text,
    51_200
)]
#[case::segments_zip_512b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Zip,
    512
)]
#[case::segments_zip_4096b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Zip,
    4_096
)]
#[case::segments_zip_51200b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Zip,
    51_200
)]
#[case::segments_pdf_512b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Pdf,
    512
)]
#[case::segments_pdf_4096b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Pdf,
    4_096
)]
#[case::segments_pdf_51200b(
    crate::common::iterators::Reproducer::Segments,
    crate::common::rng::Kind::Pdf,
    51_200
)]
#[case::lines_text_512b(
    crate::common::iterators::Reproducer::Lines,
    crate::common::rng::Kind::Text,
    512
)]
#[case::lines_text_4096b(
    crate::common::iterators::Reproducer::Lines,
    crate::common::rng::Kind::Text,
    4_096
)]
#[case::lines_text_51200b(
    crate::common::iterators::Reproducer::Lines,
    crate::common::rng::Kind::Text,
    51_200
)]
fn flows(
    #[case] reproducer: crate::common::iterators::Reproducer,
    #[case] kind: crate::common::rng::Kind,
    #[case] size: usize,
) {
}
