use rstest_reuse::template;

#[template]
#[rstest]
#[case::truncated_inside_the_window(crate::common::constants::TEST_LINE.repeat(crate::common::constants::TEST_TRACKED_COPIES), crate::common::constants::TEST_LINE.repeat(2))]
#[case::truncated_inside_a_wide_window(crate::common::constants::TEST_LINE.repeat(crate::common::constants::TEST_LARGE_COPIES), crate::common::constants::TEST_LINE.repeat(250))]
#[case::truncated_with_a_changed_prefix(crate::common::constants::TEST_LINE.repeat(crate::common::constants::TEST_LARGE_COPIES), b"bar\n".repeat(1250))]
#[case::grown_with_a_changed_prefix(crate::common::constants::TEST_LINE.repeat(crate::common::constants::TEST_TRACKED_COPIES), b"bar\n".repeat(crate::common::constants::TEST_TRACKED_COPIES + 2))]
#[case::replaced_at_the_same_size(crate::common::constants::TEST_LINE.repeat(crate::common::constants::TEST_TRACKED_COPIES), b"bar\n".repeat(crate::common::constants::TEST_TRACKED_COPIES))]
#[case::emptied(crate::common::constants::TEST_LINE.repeat(crate::common::constants::TEST_TRACKED_COPIES), Vec::new())]
#[case::truncated_at_the_window(b"a".repeat(crate::common::constants::TEST_WINDOW), b"a".repeat(crate::common::constants::TEST_WINDOW - 1))]
fn lost_prefixes(#[case] before: Vec<u8>, #[case] after: Vec<u8>) {}
