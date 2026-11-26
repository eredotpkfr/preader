from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

from common.constants import TEST_ALPHABET, TEST_INVALID_UTF8, TEST_STATE_NAME
from preader import IteratorOptions, PReader


@pytest.fixture
def data_file(make_file: Callable[..., Path]) -> Path:
    return make_file(TEST_ALPHABET)


def test_byte_iterator_repr(
    reader: PReader, tmp_file: Path, expected_repr: Callable[..., str]
) -> None:
    iterator = reader.bytes(tmp_file)

    assert repr(iterator) == expected_repr("ByteIterator", state=iterator.state)


def test_yields_every_byte_value(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(bytes(range(256)))

    assert list(reader.bytes(path)) == [bytes([value]) for value in range(256)]


def test_position_counts_finished_items(reader: PReader, data_file: Path) -> None:
    iterator = reader.bytes(data_file)

    next(iterator)

    assert iterator.state.position == 0
    assert iterator.percent() == 0.0

    next(iterator)

    assert iterator.state.position == 1


def test_threshold_records_every_item_boundary(
    make_reader: Callable[..., PReader], data_file: Path
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)

    next(iterator)

    saved = [reader.states[TEST_STATE_NAME].position for _ in iterator]

    assert saved == list(range(1, len(TEST_ALPHABET)))


def test_manual_save_does_not_reset_autosave_baseline(
    make_reader: Callable[..., PReader], data_file: Path, consume: Callable[..., Any]
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=10)
    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)

    consume(iterator, 5).save()

    assert reader.states[TEST_STATE_NAME].position == 4

    consume(iterator, 6)

    assert reader.states[TEST_STATE_NAME].position == 10


def test_file_growth_during_iteration_is_ignored(
    reader: PReader,
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(TEST_ALPHABET[:3])
    iterator = reader.bytes(path)

    next(iterator)
    append(path, b"more")

    assert list(iterator) == [TEST_ALPHABET[1:2], TEST_ALPHABET[2:3]]
    assert iterator.state.position == 3


def test_read_yields_invalid_bytes(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    invalid = b"foo" + TEST_INVALID_UTF8 + b"bar"
    iterator = reader.bytes(make_file(invalid))

    assert b"".join(iterator) == invalid
    assert iterator.state.position == len(invalid)
    assert iterator.percent() == 100.0


def test_resumed_read_floors_the_last_saved_position(
    reader: PReader,
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
) -> None:
    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)
    state = consume(iterator, 8).save()

    saving = make_reader(auto_save_state=True, auto_save_state_bytes=3)
    resumed = saving.bytes(data_file, state=state)

    consume(resumed, 3)

    assert saving.states[TEST_STATE_NAME].position == 9


def test_resume_applies_a_skip_past_the_saved_position(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)
    state = consume(iterator, 4).save()

    options = IteratorOptions(start=1, skip=10)
    resumed = list(reader.bytes(data_file, state=state, options=options))

    assert resumed == [bytes([byte]) for byte in TEST_ALPHABET[11:]]


def test_threshold_saves_during_iteration(
    make_reader: Callable[..., PReader], data_file: Path
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=10)
    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)

    saved = [
        reader.states[TEST_STATE_NAME].position
        for _ in iterator
        if TEST_STATE_NAME in reader.states
    ]

    assert list(dict.fromkeys(saved)) == [10, 20]
    assert reader.states[TEST_STATE_NAME].position == len(TEST_ALPHABET)
