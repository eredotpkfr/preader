from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

from common.constants import (
    TEST_ALPHABET,
    TEST_CHUNK_SIZE,
    TEST_INVALID_UTF8,
    TEST_LINE,
    TEST_STATE_NAME,
)
from preader import Config, IteratorOptions, PReader


def _chunked(content: bytes, size: int) -> list[bytes]:
    return [content[index : index + size] for index in range(0, len(content), size)]


@pytest.fixture
def data_file(make_file: Callable[..., Path]) -> Path:
    return make_file(TEST_ALPHABET)


@pytest.mark.parametrize(
    "drop_partial", [True, False], ids=["drop_partial", "keep_partial"]
)
def test_chunk_iterator_attributes(
    reader: PReader, tmp_file: Path, drop_partial: bool
) -> None:
    iterator = reader.chunks(tmp_file, chunk_size=3, drop_partial=drop_partial)

    assert iterator.chunk_size == 3
    assert iterator.drop_partial is drop_partial


def test_chunk_iterator_defaults(reader: PReader, tmp_file: Path) -> None:
    iterator = reader.chunks(tmp_file)

    assert iterator.chunk_size == 1024
    assert iterator.drop_partial is False


def test_chunk_iterator_repr(
    reader: PReader, tmp_file: Path, expected_repr: Callable[..., str]
) -> None:
    iterator = reader.chunks(tmp_file, chunk_size=3, drop_partial=True)

    assert repr(iterator) == expected_repr(
        "ChunkIterator",
        state=iterator.state,
        chunk_size=iterator.chunk_size,
        drop_partial=iterator.drop_partial,
    )


def test_zero_chunk_size_yields_nothing(reader: PReader, tmp_file: Path) -> None:
    assert list(reader.chunks(tmp_file, chunk_size=0)) == []


def test_drop_partial_yields_nothing_for_an_empty_file(
    reader: PReader, empty_file: Path
) -> None:
    assert list(reader.chunks(empty_file, drop_partial=True)) == []


def test_drop_partial_yields_nothing_for_a_large_chunk_size(
    reader: PReader, tmp_file: Path
) -> None:
    assert list(reader.chunks(tmp_file, chunk_size=1024, drop_partial=True)) == []


@pytest.mark.parametrize("chunk_size", [1024, 2**64 - 1], ids=["large", "largest"])
def test_yields_one_chunk_for_a_large_chunk_size(
    reader: PReader, tmp_file: Path, chunk_size: int
) -> None:
    assert list(reader.chunks(tmp_file, chunk_size=chunk_size)) == [TEST_LINE]


@pytest.mark.parametrize("chunk_size", ["3", 1.5, None], ids=["str", "float", "none"])
def test_raises_when_the_chunk_size_is_not_an_integer(
    reader: PReader, tmp_file: Path, chunk_size: object
) -> None:
    with pytest.raises(TypeError, match="cannot be interpreted as an integer"):
        reader.chunks(tmp_file, chunk_size=chunk_size)  # type: ignore[arg-type]


@pytest.mark.parametrize(
    ("chunk_size", "message"),
    [(-1, "negative int"), (2**64, "too big")],
    ids=["negative", "too_large"],
)
def test_raises_when_the_chunk_size_overflows(
    reader: PReader, tmp_file: Path, chunk_size: int, message: str
) -> None:
    with pytest.raises(OverflowError, match=message):
        reader.chunks(tmp_file, chunk_size=chunk_size)


def test_drop_partial_discards_a_single_byte_file(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"a")

    assert list(reader.chunks(path, drop_partial=True)) == []


def test_splits_into_fixed_size_pieces(reader: PReader, data_file: Path) -> None:
    chunks = list(reader.chunks(data_file, chunk_size=3))

    assert chunks == _chunked(TEST_ALPHABET, 3)


def test_whole_file_chunk_size_yields_one_chunk(
    reader: PReader, data_file: Path
) -> None:
    size = len(TEST_ALPHABET)

    assert list(reader.chunks(data_file, chunk_size=size)) == [TEST_ALPHABET]


@pytest.mark.parametrize(
    ("options", "expected"),
    [
        (IteratorOptions(start=6), TEST_ALPHABET[6:]),
        (IteratorOptions(end=10), TEST_ALPHABET[:10]),
        (IteratorOptions(skip=4), TEST_ALPHABET[12:]),
        (IteratorOptions(start=6, limit=2), TEST_ALPHABET[6:12]),
    ],
    ids=["start", "end", "skip", "start_and_limit"],
)
def test_options_narrow_the_output(
    reader: PReader, data_file: Path, options: IteratorOptions, expected: bytes
) -> None:
    chunks = _chunked(expected, 3)

    assert list(reader.chunks(data_file, options=options, chunk_size=3)) == chunks


def test_limit_counts_chunks_from_an_unaligned_start(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(start=1, limit=1)

    assert list(reader.chunks(data_file, options=options, chunk_size=4)) == [b"bcde"]


def test_skip_saturates_when_multiplied_by_the_chunk_size(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(skip=2**32)

    assert list(reader.chunks(data_file, options=options, chunk_size=2**32)) == []


def test_start_is_not_realigned_to_the_chunk_grid(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(start=7)

    iterator = reader.chunks(data_file, options=options, chunk_size=3)

    assert list(iterator) == _chunked(TEST_ALPHABET[7:], 3)


def test_drop_partial_discards_the_partial_chunk(
    reader: PReader, data_file: Path
) -> None:
    chunks = list(reader.chunks(data_file, chunk_size=3, drop_partial=True))

    assert chunks == _chunked(TEST_ALPHABET[: len(TEST_ALPHABET) // 3 * 3], 3)


def test_chunk_size_of_one_yields_single_bytes(
    reader: PReader, data_file: Path
) -> None:
    chunks = list(reader.chunks(data_file, chunk_size=1))

    assert chunks == [bytes([value]) for value in TEST_ALPHABET]
    assert chunks == list(reader.bytes(data_file))


def test_drop_partial_keeps_an_exact_chunk(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(TEST_ALPHABET[:6])
    iterator = reader.chunks(path, chunk_size=3, drop_partial=True)

    assert list(iterator) == [TEST_ALPHABET[:3], TEST_ALPHABET[3:6]]


def test_yields_every_byte_value(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(bytes(range(256)))

    assert list(reader.chunks(path, chunk_size=1)) == [
        bytes([value]) for value in range(256)
    ]


def test_keeps_a_partial_chunk_by_default(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(end=12)

    chunks = list(reader.chunks(data_file, options=options, chunk_size=5))

    assert chunks == [TEST_ALPHABET[0:5], TEST_ALPHABET[5:10], TEST_ALPHABET[10:12]]


def test_drop_partial_discards_a_chunk_cut_short_by_end(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(end=12)

    iterator = reader.chunks(
        data_file, options=options, chunk_size=5, drop_partial=True
    )

    assert list(iterator) == [TEST_ALPHABET[0:5], TEST_ALPHABET[5:10]]
    assert iterator.state.position == 10


def test_resume_applies_a_different_chunk_size(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=4)
    state = consume(iterator, 2).save()

    resumed = list(reader.chunks(data_file, state=state, chunk_size=3))

    assert resumed == _chunked(TEST_ALPHABET[4:], 3)


def test_drop_partial_reads_past_a_buffer_refill(
    make_reader: Callable[..., PReader], make_file: Callable[..., Path]
) -> None:
    reader = make_reader(buffer_capacity=64)
    content = TEST_ALPHABET * 20
    iterator = reader.chunks(
        make_file(content), chunk_size=TEST_CHUNK_SIZE, drop_partial=True
    )

    assert (
        b"".join(iterator) == content[: len(content) - len(content) % TEST_CHUNK_SIZE]
    )


def test_position_counts_finished_items(reader: PReader, data_file: Path) -> None:
    iterator = reader.chunks(data_file, chunk_size=3)

    next(iterator)

    assert iterator.state.position == 0
    assert iterator.percent() == 0.0

    next(iterator)

    assert iterator.state.position == 3


def test_drop_partial_discards_a_truncated_chunk(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    truncate: Callable[[Path, int], None],
) -> None:
    reader = make_reader(buffer_capacity=1)
    path = make_file(b"foobarbaz")
    iterator = reader.chunks(path, chunk_size=3, drop_partial=True)

    assert next(iterator) == b"foo"

    truncate(path, 5)

    assert list(iterator) == []
    assert iterator.state.position == 5


def test_file_growth_during_iteration_is_ignored(
    reader: PReader,
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(TEST_ALPHABET[:6])
    iterator = reader.chunks(path, chunk_size=3)

    next(iterator)
    append(path, b"more")

    assert list(iterator) == [TEST_ALPHABET[3:6]]
    assert iterator.state.position == 6


def test_read_yields_invalid_bytes(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    invalid = b"foo" + TEST_INVALID_UTF8 + b"bar"
    iterator = reader.chunks(make_file(invalid), chunk_size=4)

    assert list(iterator) == [invalid[:4], invalid[4:]]
    assert iterator.state.position == len(invalid)
    assert iterator.percent() == 100.0


def test_resumed_read_floors_the_last_saved_position(
    reader: PReader,
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
) -> None:
    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=2)
    state = consume(iterator, 4).save()

    saving = make_reader(auto_save_state=True, auto_save_state_bytes=4)
    resumed = saving.chunks(data_file, state=state, chunk_size=2)

    consume(resumed, 2)

    assert saving.states[TEST_STATE_NAME].position == 8


def test_resume_applies_a_different_chunk_size_and_limit(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(limit=1)
    iterator = reader.chunks(
        data_file, state=TEST_STATE_NAME, options=options, chunk_size=4
    )

    list(iterator)

    state = iterator.save()
    resumed = reader.chunks(
        data_file, state=state, options=IteratorOptions(limit=2), chunk_size=2
    )

    assert list(resumed) == [b"ef", b"gh"]


def test_resume_applies_a_skip_past_the_saved_position(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=2)
    state = consume(iterator, 2).save()

    options = IteratorOptions(start=1, skip=5)
    resumed = list(reader.chunks(data_file, state=state, chunk_size=2, options=options))

    assert resumed == _chunked(TEST_ALPHABET[11:], 2)


def test_raises_when_a_save_fails_on_a_chunk_dropped_by_end(
    make_reader: Callable[..., PReader], data_file: Path, config: Config
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    reader = make_reader(auto_save_state=True)
    options = IteratorOptions(end=12)
    iterator = reader.chunks(
        data_file,
        state=TEST_STATE_NAME,
        options=options,
        chunk_size=8,
        drop_partial=True,
    )

    assert next(iterator) == TEST_ALPHABET[:8]

    with pytest.raises(FileExistsError):
        next(iterator)

    assert iterator.state.position == 8


def test_raises_when_a_save_fails_on_a_dropped_short_chunk(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    truncate: Callable[[Path, int], None],
    config: Config,
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    reader = make_reader(auto_save_state=True, buffer_capacity=1)
    path = make_file(b"foobarbaz")
    iterator = reader.chunks(
        path, state=TEST_STATE_NAME, chunk_size=3, drop_partial=True
    )

    assert next(iterator) == b"foo"

    truncate(path, 5)

    with pytest.raises(FileExistsError):
        next(iterator)

    assert iterator.state.position == 5


def test_threshold_saves_during_iteration(
    make_reader: Callable[..., PReader], data_file: Path
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=10)
    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3)

    saved = [
        reader.states[TEST_STATE_NAME].position
        for _ in iterator
        if TEST_STATE_NAME in reader.states
    ]

    assert list(dict.fromkeys(saved)) == [12, 24]
    assert reader.states[TEST_STATE_NAME].position == len(TEST_ALPHABET)
