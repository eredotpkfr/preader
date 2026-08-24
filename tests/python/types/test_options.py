import pytest

from preader import IteratorOptions

from constants import TEST_ALPHABET


def test_iterator_options_defaults():
    options = IteratorOptions()

    assert options.start == 0
    assert options.end == 2**64 - 1
    assert options.skip == 0
    assert options.limit == 2**64 - 1


def test_iterator_options_mutation_affects_reads(reader, make_file):
    path = make_file(TEST_ALPHABET)
    options = IteratorOptions(start=2)

    assert b"".join(reader.bytes(path, options=options)) == TEST_ALPHABET[2:]

    options.start = 5
    assert b"".join(reader.bytes(path, options=options)) == TEST_ALPHABET[5:]


@pytest.mark.parametrize(
    "value",
    [-1, -2, -100, -(2**31), -(2**63)],
    ids=["minus_one", "minus_two", "minus_hundred", "i32_min", "i64_min"],
)
def test_iterator_options_raises_when_negative(value):
    with pytest.raises((OverflowError, TypeError)):
        IteratorOptions(start=value)


def test_iterator_options_repr(expected_repr):
    options = IteratorOptions(start=10, end=100, skip=5, limit=50)

    assert repr(options) == expected_repr(
        "IteratorOptions",
        start=options.start,
        end=options.end,
        skip=options.skip,
        limit=options.limit,
    )
