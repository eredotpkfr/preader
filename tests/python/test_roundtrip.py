import itertools
import math

from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

from cases.roundtrip import ARCHIVES, CONTENTS, FLOWS, TEXTS, WINDOWS
from common.constants import TEST_CHUNK_SIZE, TEST_DEFAULT_DELIMITER, TEST_STATE_NAME
from common.rng import seeded_content
from preader import Config, IteratorOptions, PReader

CHUNK_SIZES = [1, 4096]
BUFFER_CAPACITIES = [1024, Config().buffer_capacity]
CYCLES = 5
STREAM_DELIMITERS = {"comma": TEST_DEFAULT_DELIMITER, "newline": b"\n", "null": b"\x00"}


def _split_stream(
    content: bytes, delimiter: bytes, *, keep: bool = False
) -> list[bytes]:
    segments = content.split(delimiter)
    tail = segments.pop()
    kept = [segment + delimiter for segment in segments] if keep else segments

    return [*kept, tail] if tail else kept


@pytest.mark.parametrize(("make_iterator", "kind", "size"), WINDOWS)
def test_iterators_reproduce_the_content(
    reader: PReader,
    make_file: Callable[..., Path],
    seed: int,
    make_iterator: Callable[..., Any],
    kind: str,
    size: int,
) -> None:
    content = seeded_content(kind, size, seed)

    assert b"".join(make_iterator(reader, make_file(content))) == content


@pytest.mark.parametrize("chunk_size", CHUNK_SIZES)
@pytest.mark.parametrize(("kind", "size"), CONTENTS)
def test_chunks_reproduce_the_content(
    reader: PReader,
    make_file: Callable[..., Path],
    seed: int,
    chunk_size: int,
    kind: str,
    size: int,
) -> None:
    content = seeded_content(kind, size, seed)

    assert b"".join(reader.chunks(make_file(content), chunk_size=chunk_size)) == content


@pytest.mark.parametrize(("kind", "size"), ARCHIVES)
def test_lines_raises_when_the_content_is_binary(
    reader: PReader, make_file: Callable[..., Path], seed: int, kind: str, size: int
) -> None:
    path = make_file(seeded_content(kind, size, seed))

    with pytest.raises(ValueError, match="invalid utf-8"):
        list(reader.lines(path))


@pytest.mark.parametrize(
    ("text", "encoding", "delimiter"),
    [
        ("café,鉄", "utf-16-le", TEST_DEFAULT_DELIMITER),
        ("café,鉄", "utf-16-be", TEST_DEFAULT_DELIMITER),
        ("ਊ", "utf-16-le", b"\n"),
    ],
    ids=["little_endian", "big_endian", "newline_inside_a_character"],
)
def test_segments_split_utf16_on_bytes(
    reader: PReader,
    make_file: Callable[..., Path],
    text: str,
    encoding: str,
    delimiter: bytes,
) -> None:
    content = text.encode(encoding)

    segments = list(reader.segments(make_file(content), delimiter=delimiter))

    assert segments == _split_stream(content, delimiter)


@pytest.mark.parametrize(("make_iterator", "kind", "size"), WINDOWS)
def test_options_window_the_byte_range(
    reader: PReader,
    make_file: Callable[..., Path],
    seed: int,
    make_iterator: Callable[..., Any],
    kind: str,
    size: int,
) -> None:
    content = seeded_content(kind, size, seed)
    options = IteratorOptions(start=100, end=500)

    windowed = make_iterator(reader, make_file(content), options=options)

    assert b"".join(windowed) == content[100:500]


@pytest.mark.parametrize(("kind", "size"), CONTENTS)
def test_segments_skip_empty_drops_the_blank_segments(
    reader: PReader, make_file: Callable[..., Path], seed: int, kind: str, size: int
) -> None:
    content = seeded_content(kind, size, seed)

    kept = list(reader.segments(make_file(content), skip_empty=True))
    segments = _split_stream(content, TEST_DEFAULT_DELIMITER)

    assert kept == [segment for segment in segments if segment]


@pytest.mark.parametrize(("kind", "size"), TEXTS)
def test_skip_empty_drops_the_blank_lines(
    reader: PReader, make_file: Callable[..., Path], seed: int, kind: str, size: int
) -> None:
    content = seeded_content(kind, size, seed)

    kept = list(reader.lines(make_file(content), skip_empty=True))

    assert kept == [line for line in content.decode().split("\n")[:-1] if line]


@pytest.mark.parametrize("buffer_capacity", BUFFER_CAPACITIES)
@pytest.mark.parametrize(("kind", "size"), CONTENTS)
def test_drop_partial_stops_at_the_last_whole_chunk(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    seed: int,
    buffer_capacity: int,
    kind: str,
    size: int,
) -> None:
    content = seeded_content(kind, size, seed)

    reader = make_reader(buffer_capacity=buffer_capacity)
    iterator = reader.chunks(
        make_file(content), chunk_size=TEST_CHUNK_SIZE, drop_partial=True
    )
    expected = content[: len(content) - len(content) % TEST_CHUNK_SIZE]

    assert b"".join(iterator) == expected


@pytest.mark.parametrize("buffer_capacity", BUFFER_CAPACITIES)
@pytest.mark.parametrize(("kind", "size"), TEXTS)
def test_lines_reproduce_the_text(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    seed: int,
    buffer_capacity: int,
    kind: str,
    size: int,
) -> None:
    content = seeded_content(kind, size, seed)

    reader = make_reader(buffer_capacity=buffer_capacity)
    path = make_file(content)
    text = content.decode()

    assert "".join(reader.lines(path, keepends=True)) == text
    assert list(reader.lines(path)) == text.split("\n")[:-1]


@pytest.mark.parametrize(("make_iterator", "kind", "size"), FLOWS)
def test_percent_tracks_the_position(
    reader: PReader,
    make_file: Callable[..., Path],
    seed: int,
    make_iterator: Callable[..., Any],
    kind: str,
    size: int,
) -> None:
    content = seeded_content(kind, size, seed)

    iterator = make_iterator(reader, make_file(content))

    for _ in iterator:
        expected = iterator.state.position * 100 / len(content)

        assert iterator.percent() == expected

    assert iterator.state.position == len(content)
    assert iterator.percent() == 100.0


@pytest.mark.parametrize("buffer_capacity", BUFFER_CAPACITIES)
@pytest.mark.parametrize(
    "delimiter", list(STREAM_DELIMITERS.values()), ids=list(STREAM_DELIMITERS)
)
@pytest.mark.parametrize(("kind", "size"), CONTENTS)
def test_segments_split_like_a_stream(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    seed: int,
    buffer_capacity: int,
    delimiter: bytes,
    kind: str,
    size: int,
) -> None:
    content = seeded_content(kind, size, seed)

    reader = make_reader(buffer_capacity=buffer_capacity)
    path = make_file(content)

    stripped = list(reader.segments(path, delimiter=delimiter))
    kept = list(reader.segments(path, delimiter=delimiter, keep_delimiter=True))

    assert stripped == _split_stream(content, delimiter)
    assert kept == _split_stream(content, delimiter, keep=True)


@pytest.mark.parametrize(
    "delimiter", list(STREAM_DELIMITERS.values()), ids=list(STREAM_DELIMITERS)
)
@pytest.mark.parametrize(("kind", "size"), CONTENTS)
def test_segments_windows_cover_the_content_once(
    reader: PReader,
    make_file: Callable[..., Path],
    seed: int,
    delimiter: bytes,
    kind: str,
    size: int,
) -> None:
    content = seeded_content(kind, size, seed)

    path = make_file(content)
    cuts = [0, len(content) // 3, len(content) * 2 // 3, len(content)]

    segments = [
        segment
        for start, end in itertools.pairwise(cuts)
        for segment in reader.segments(
            path,
            options=IteratorOptions(start=start, end=end),
            delimiter=delimiter,
            keep_delimiter=True,
            align_start=start > 0,
        )
    ]

    assert b"".join(segments) == content


@pytest.mark.parametrize(("make_iterator", "kind", "size"), FLOWS)
def test_resume_rebuilds_the_file_in_cycles(
    reader: PReader,
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    seed: int,
    make_iterator: Callable[..., Any],
    kind: str,
    size: int,
) -> None:
    autosaving = make_reader(auto_save_state=True)
    path = make_file(seeded_content(kind, size, seed))

    expected = list(make_iterator(reader, path))
    cycle = math.ceil(len(expected) / (CYCLES - 1)) + 1

    rebuilt = [
        item
        for _ in range(CYCLES)
        for item in itertools.islice(
            make_iterator(autosaving, path, state=TEST_STATE_NAME), cycle
        )
    ]

    assert rebuilt == [
        item
        for index in range(CYCLES)
        for item in expected[index * (cycle - 1) :][:cycle]
    ]
