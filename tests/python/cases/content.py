import pytest

from common.constants import (
    TEST_LARGE_COPIES,
    TEST_LINE,
    TEST_TRACKED_COPIES,
    TEST_WINDOW,
)

LOST_PREFIXES = [
    pytest.param(
        TEST_LINE * TEST_TRACKED_COPIES, TEST_LINE * 2, id="truncated_inside_the_window"
    ),
    pytest.param(
        TEST_LINE * TEST_LARGE_COPIES,
        TEST_LINE * 250,
        id="truncated_inside_a_wide_window",
    ),
    pytest.param(
        TEST_LINE * TEST_LARGE_COPIES,
        b"bar\n" * 1250,
        id="truncated_with_a_changed_prefix",
    ),
    pytest.param(
        TEST_LINE * TEST_TRACKED_COPIES,
        b"bar\n" * (TEST_TRACKED_COPIES + 2),
        id="grown_with_a_changed_prefix",
    ),
    pytest.param(
        TEST_LINE * TEST_TRACKED_COPIES,
        b"bar\n" * TEST_TRACKED_COPIES,
        id="replaced_at_the_same_size",
    ),
    pytest.param(TEST_LINE * TEST_TRACKED_COPIES, b"", id="emptied"),
    pytest.param(
        b"a" * TEST_WINDOW, b"a" * (TEST_WINDOW - 1), id="truncated_at_the_window"
    ),
]
