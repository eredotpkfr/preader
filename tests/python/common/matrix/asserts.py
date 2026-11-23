from collections.abc import Callable, Iterable

import pytest

from preader import IteratorOptions


def assert_reference[T](
    read: Callable[[], Iterable[T]], options: IteratorOptions, expected: list[T] | None
) -> None:
    if options.start > options.end:
        with pytest.raises(ValueError, match="start must not be greater than end"):
            read()

    elif expected is None:
        with pytest.raises(ValueError, match="invalid utf-8"):
            list(read())

    else:
        assert list(read()) == expected
