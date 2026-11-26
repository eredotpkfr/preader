from collections.abc import Callable

import pytest

from preader import IteratorOptions


def test_iterator_options_defaults() -> None:
    options = IteratorOptions()

    assert options.start == 0
    assert options.end == 2**64 - 1
    assert options.skip == 0
    assert options.limit == 2**64 - 1


def test_iterator_options_fields() -> None:
    options = IteratorOptions(start=1, end=2, skip=3, limit=4)

    assert options.start == 1
    assert options.end == 2
    assert options.skip == 3
    assert options.limit == 4


def test_iterator_options_compares_by_value() -> None:
    options = IteratorOptions(start=10, end=100)

    assert options == IteratorOptions(start=10, end=100)
    assert options != IteratorOptions(start=10, end=101)


def test_iterator_options_repr(expected_repr: Callable[..., str]) -> None:
    options = IteratorOptions(start=10, end=100, skip=5, limit=50)

    assert repr(options) == expected_repr(
        "IteratorOptions",
        start=options.start,
        end=options.end,
        skip=options.skip,
        limit=options.limit,
    )


def test_iterator_options_raises_when_an_argument_is_positional() -> None:
    with pytest.raises(TypeError, match="positional"):
        IteratorOptions(1)  # type: ignore[call-arg]


@pytest.mark.parametrize("field", ["start", "end", "skip", "limit"])
@pytest.mark.parametrize(
    ("value", "message"),
    [
        (-1, "negative int"),
        (-2, "negative int"),
        (-100, "negative int"),
        (-(2**31), "negative int"),
        (-(2**63), "negative int"),
        (2**64, "too big"),
    ],
    ids=["minus_one", "minus_two", "minus_hundred", "i32_min", "i64_min", "too_large"],
)
def test_iterator_options_raises_when_a_field_overflows(
    field: str, value: int, message: str
) -> None:
    with pytest.raises(OverflowError, match=message):
        IteratorOptions(**{field: value})


@pytest.mark.parametrize("field", ["end", "limit", "skip", "start"])
def test_iterator_options_raises_when_a_field_is_set(field: str) -> None:
    options = IteratorOptions()

    with pytest.raises(AttributeError, match="is not writable"):
        setattr(options, field, None)
