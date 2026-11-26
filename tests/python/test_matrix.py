import functools

from collections.abc import Callable
from pathlib import Path

import pytest

from hypothesis import given
from hypothesis import strategies as st

from cases.delimiter import DELIMITERS
from common.matrix import reference, strategies
from common.matrix.asserts import assert_reference
from preader import IteratorOptions, PReader


@given(drawn=strategies.bytes_())
def test_bytes_match_the_reference(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    drawn: tuple[bytes, IteratorOptions, int],
) -> None:
    content, options, capacity = drawn
    read = functools.partial(
        make_reader(buffer_capacity=capacity).bytes, make_file(content), options=options
    )

    assert_reference(read, options, reference.bytes_(content, options))


@pytest.mark.parametrize("drop_partial", [False, True])
@given(drawn=strategies.chunks())
def test_chunks_match_the_reference(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    drop_partial: bool,
    drawn: tuple[bytes, IteratorOptions, int, int],
) -> None:
    content, options, size, capacity = drawn
    read = functools.partial(
        make_reader(buffer_capacity=capacity).chunks,
        make_file(content),
        options=options,
        chunk_size=size,
        drop_partial=drop_partial,
    )

    assert_reference(
        read,
        options,
        reference.chunks(content, size, options, drop_partial=drop_partial),
    )


@pytest.mark.parametrize("keep", [False, True])
@pytest.mark.parametrize("skip_empty", [False, True])
@pytest.mark.parametrize("align", [False, True])
@given(drawn=strategies.lines())
def test_lines_match_the_reference(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    keep: bool,
    skip_empty: bool,
    align: bool,
    drawn: tuple[bytes, IteratorOptions, int],
) -> None:
    content, options, capacity = drawn
    read = functools.partial(
        make_reader(buffer_capacity=capacity).lines,
        make_file(content),
        options=options,
        keepends=keep,
        skip_empty=skip_empty,
        align_start=align,
    )

    assert_reference(
        read,
        options,
        reference.lines(content, reference.Shape(keep, skip_empty, align), options),
    )


@pytest.mark.parametrize("delimiter", DELIMITERS)
@pytest.mark.parametrize("keep", [False, True])
@pytest.mark.parametrize("skip_empty", [False, True])
@pytest.mark.parametrize("align", [False, True])
@given(data=st.data())
def test_segments_match_the_reference(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    delimiter: int,
    keep: bool,
    skip_empty: bool,
    align: bool,
    data: st.DataObject,
) -> None:
    content, options, capacity = data.draw(strategies.segments(delimiter))
    read = functools.partial(
        make_reader(buffer_capacity=capacity).segments,
        make_file(content),
        options=options,
        delimiter=bytes([delimiter]),
        keep_delimiter=keep,
        skip_empty=skip_empty,
        align_start=align,
    )

    assert_reference(
        read,
        options,
        reference.segments(
            content, reference.Shape(keep, skip_empty, align), options, delimiter
        ),
    )
