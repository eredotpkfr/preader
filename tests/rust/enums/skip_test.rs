use preader::SkipPlan;
use rstest::rstest;

use crate::common::constants::TEST_DEFAULT_DELIMITER;

#[rstest]
#[case::bytes(SkipPlan::Bytes(3), (3, 0))]
#[case::items(SkipPlan::Items { count: 3, boundary: None }, (0, 3))]
#[case::aligned_items(SkipPlan::Items { count: 3, boundary: Some(TEST_DEFAULT_DELIMITER) }, (0, 3))]
fn counts_split_bytes_from_items(#[case] plan: SkipPlan, #[case] expected: (u64, u64)) {
    assert_eq!(plan.counts(), expected);
}

#[rstest]
#[case::bytes(SkipPlan::Bytes(3), None)]
#[case::items(SkipPlan::Items { count: 3, boundary: None }, None)]
#[case::aligned_items(SkipPlan::Items { count: 3, boundary: Some(TEST_DEFAULT_DELIMITER) }, Some(TEST_DEFAULT_DELIMITER))]
fn boundary_is_set_only_for_aligned_items(#[case] plan: SkipPlan, #[case] expected: Option<u8>) {
    assert_eq!(plan.boundary(), expected);
}
