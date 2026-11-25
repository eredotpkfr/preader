from collections.abc import Callable, Sequence
from pathlib import Path
from typing import Any

import pytest

from cases.delimiter import DELIMITERS
from common.constants import (
    TEST_DEFAULT_DELIMITER,
    TEST_INVALID_UTF8,
    TEST_STATE_NAME,
    TEST_UNICODE_TEXT,
)
from preader import Config, IteratorOptions, PReader

SEGMENTS = [f"seg-{i}".encode() for i in range(10)]
SEGMENT_CONTENT = TEST_DEFAULT_DELIMITER.join(SEGMENTS)
BLANK_SEGMENT_CONTENT = TEST_DEFAULT_DELIMITER.join(
    [SEGMENTS[0], b"", SEGMENTS[2], b""]
)


@pytest.fixture
def data_file(make_file: Callable[..., Path]) -> Path:
    return make_file(SEGMENT_CONTENT)


@pytest.mark.parametrize(
    ("keep_delimiter", "skip_empty"),
    [(True, True), (True, False), (False, True), (False, False)],
    ids=["both", "keep_delimiter", "skip_empty", "neither"],
)
def test_segment_iterator_attributes(
    reader: PReader, tmp_file: Path, keep_delimiter: bool, skip_empty: bool
) -> None:
    options = IteratorOptions(skip=1)

    iterator = reader.segments(
        tmp_file,
        options=options,
        delimiter=b";",
        keep_delimiter=keep_delimiter,
        skip_empty=skip_empty,
    )

    assert iterator.delimiter == b";"
    assert iterator.keep_delimiter is keep_delimiter
    assert iterator.skip_empty is skip_empty
    assert iterator.skip_remaining == options.skip


def test_segment_iterator_defaults(reader: PReader, tmp_file: Path) -> None:
    iterator = reader.segments(tmp_file)

    assert iterator.delimiter == TEST_DEFAULT_DELIMITER
    assert iterator.keep_delimiter is False
    assert iterator.skip_empty is False
    assert iterator.skip_remaining == 0


def test_segment_iterator_repr(
    reader: PReader, tmp_file: Path, expected_repr: Callable[..., str]
) -> None:
    iterator = reader.segments(tmp_file, keep_delimiter=True, skip_empty=True)

    assert repr(iterator) == expected_repr(
        "SegmentIterator",
        state=iterator.state,
        delimiter=iterator.delimiter,
        keep_delimiter=iterator.keep_delimiter,
        skip_empty=iterator.skip_empty,
        skip_remaining=iterator.skip_remaining,
    )


def test_splits_on_the_delimiter(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    assert list(reader.segments(make_file(b"foo,bar,baz"))) == [b"foo", b"bar", b"baz"]


@pytest.mark.parametrize(
    "delimiter",
    [b"", b",,", "é".encode(), ",".encode("utf-16-le")],
    ids=["empty", "two_bytes", "utf8_character", "utf16_character"],
)
def test_raises_when_the_delimiter_is_not_a_single_byte(
    reader: PReader, tmp_file: Path, delimiter: bytes
) -> None:
    with pytest.raises(ValueError, match="sequence of length 1"):
        reader.segments(tmp_file, delimiter=delimiter)


@pytest.mark.parametrize("delimiter", [[-1], [256]], ids=["negative", "above_255"])
def test_raises_when_the_delimiter_byte_is_out_of_range(
    reader: PReader, tmp_file: Path, delimiter: list[int]
) -> None:
    with pytest.raises(OverflowError, match="'delimiter'"):
        reader.segments(tmp_file, delimiter=delimiter)


@pytest.mark.parametrize(
    ("delimiter", "message"),
    [
        (TEST_DEFAULT_DELIMITER.decode(), "cannot be interpreted as an integer"),
        (TEST_DEFAULT_DELIMITER[0], "is not an instance of 'Sequence'"),
        (None, "is not an instance of 'Sequence'"),
    ],
    ids=["str", "int", "none"],
)
def test_raises_when_the_delimiter_is_not_a_byte_sequence(
    reader: PReader, tmp_file: Path, delimiter: object, message: str
) -> None:
    with pytest.raises(TypeError, match=message):
        reader.segments(tmp_file, delimiter=delimiter)  # type: ignore[arg-type]


def test_skip_remaining_is_zero_for_an_empty_file(
    reader: PReader, empty_file: Path
) -> None:
    iterator = reader.segments(empty_file, options=IteratorOptions(skip=2))

    assert iterator.skip_remaining == 0


def test_newline_delimiter_keeps_the_carriage_return(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\r\nbar\n")

    assert list(reader.segments(path, delimiter=b"\n")) == [b"foo\r", b"bar"]


def test_keep_delimiter_leaves_the_last_segment_unterminated(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo,bar")

    assert list(reader.segments(path, keep_delimiter=True)) == [b"foo,", b"bar"]


@pytest.mark.parametrize(
    "delimiter",
    [bytearray(b";"), memoryview(b";"), list(b";"), tuple(b";")],
    ids=["bytearray", "memoryview", "list", "tuple"],
)
def test_delimiter_accepts_a_byte_sequence(
    reader: PReader, make_file: Callable[..., Path], delimiter: Sequence[int]
) -> None:
    path = make_file(b"foo;bar")

    assert list(reader.segments(path, delimiter=delimiter)) == [b"foo", b"bar"]


def test_keep_delimiter_preserves_the_delimiter(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo,bar,")
    iterator = reader.segments(path, keep_delimiter=True)

    assert list(iterator) == [b"foo,", b"bar,"]


@pytest.mark.parametrize(
    ("keep_delimiter", "skip_empty", "expected"),
    [
        (False, False, [b"foo", b"", b"bar"]),
        (False, True, [b"foo", b"bar"]),
        (True, False, [b"foo,", b",", b"bar,"]),
        (True, True, [b"foo,", b"bar,"]),
    ],
    ids=["neither", "skip_empty", "keep_delimiter", "both"],
)
def test_blank_segments_follow_keep_delimiter_and_skip_empty(
    reader: PReader,
    make_file: Callable[..., Path],
    keep_delimiter: bool,
    skip_empty: bool,
    expected: list[bytes],
) -> None:
    path = make_file(b"foo,,bar,")
    iterator = reader.segments(
        path, keep_delimiter=keep_delimiter, skip_empty=skip_empty
    )

    assert list(iterator) == expected


def test_align_start_saturates_a_maximum_skip(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(start=2, skip=2**64 - 1)

    assert list(reader.segments(data_file, options=options, align_start=True)) == []


def test_content_without_the_delimiter_yields_one_segment(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    content = SEGMENT_CONTENT.replace(TEST_DEFAULT_DELIMITER, b"")
    path = make_file(content)

    assert list(reader.segments(path)) == [content]


@pytest.mark.parametrize(
    ("keep_delimiter", "skip_empty", "expected"),
    [
        (False, False, [b"", b"", b""]),
        (False, True, []),
        (True, False, [TEST_DEFAULT_DELIMITER] * 3),
        (True, True, []),
    ],
    ids=["neither", "skip_empty", "keep_delimiter", "both"],
)
def test_delimiter_only_file_follows_keep_delimiter_and_skip_empty(
    reader: PReader,
    make_file: Callable[..., Path],
    keep_delimiter: bool,
    skip_empty: bool,
    expected: list[bytes],
) -> None:
    path = make_file(TEST_DEFAULT_DELIMITER * 3)
    iterator = reader.segments(
        path, keep_delimiter=keep_delimiter, skip_empty=skip_empty
    )

    assert list(iterator) == expected


def test_align_start_skips_a_partial_segment(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(start=14)

    segments = list(reader.segments(data_file, options=options, align_start=True))

    assert segments == SEGMENTS[3:]


def test_align_start_and_skip_count_a_blank_segment(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(BLANK_SEGMENT_CONTENT)
    options = IteratorOptions(start=2, skip=1)

    assert next(reader.segments(path, options=options, align_start=True)) == SEGMENTS[2]


@pytest.mark.parametrize(
    ("options", "align_start"),
    [(IteratorOptions(skip=1), False), (IteratorOptions(start=1), True)],
    ids=["skip", "align_start"],
)
def test_skipped_items_count_as_finished(
    reader: PReader, data_file: Path, options: IteratorOptions, align_start: bool
) -> None:
    iterator = reader.segments(data_file, options=options, align_start=align_start)

    next(iterator)

    assert iterator.state.position == len(SEGMENTS[0]) + len(TEST_DEFAULT_DELIMITER)


@pytest.mark.parametrize(
    ("align_start", "keep_delimiter", "skip_empty", "expected"),
    [
        (True, True, True, [b"seg-2,"]),
        (True, True, False, [b",", b"seg-2,"]),
        (True, False, True, [b"seg-2"]),
        (True, False, False, [b"", b"seg-2"]),
        (False, True, True, [b"-0,", b"seg-2,"]),
        (False, True, False, [b"-0,", b",", b"seg-2,"]),
        (False, False, True, [b"-0", b"seg-2"]),
        (False, False, False, [b"-0", b"", b"seg-2"]),
    ],
    ids=[
        "aligned_both",
        "aligned_keep_delimiter",
        "aligned_skip_empty",
        "aligned_neither",
        "unaligned_both",
        "unaligned_keep_delimiter",
        "unaligned_skip_empty",
        "unaligned_neither",
    ],
)
def test_flag_combinations_narrow_the_output(
    reader: PReader,
    make_file: Callable[..., Path],
    align_start: bool,
    keep_delimiter: bool,
    skip_empty: bool,
    expected: list[bytes],
) -> None:
    path = make_file(BLANK_SEGMENT_CONTENT)
    options = IteratorOptions(start=3)

    iterator = reader.segments(
        path,
        options=options,
        keep_delimiter=keep_delimiter,
        align_start=align_start,
        skip_empty=skip_empty,
    )

    assert list(iterator) == expected


def test_keeps_a_partial_segment_by_default(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(start=14)

    segments = list(reader.segments(data_file, options=options))

    assert segments == [SEGMENTS[2][2:], *SEGMENTS[3:]]


def test_skipping_holds_back_an_unterminated_segment(
    reader: PReader,
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"foo,bar")
    iterator = reader.segments(path, options=IteratorOptions(skip=2))

    append(path, b"more")

    assert list(iterator) == []
    assert iterator.state.position == 4


def test_skipping_past_a_terminated_segment_ignores_growth(
    reader: PReader,
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"foo,bar,")
    iterator = reader.segments(path, options=IteratorOptions(skip=2))

    append(path, b"more")

    assert list(iterator) == []
    assert iterator.state.position == 8


def test_skipped_blank_segment_does_not_consume_the_limit(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(BLANK_SEGMENT_CONTENT)
    options = IteratorOptions(limit=2)

    iterator = reader.segments(path, options=options, skip_empty=True)

    assert list(iterator) == [SEGMENTS[0], SEGMENTS[2]]


def test_yields_every_byte_value(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    content = bytes(range(256))
    path = make_file(content)
    iterator = reader.segments(path, keep_delimiter=True)

    assert list(iterator) == [content[:45], content[45:]]


def test_growth_holds_back_an_unterminated_segment_on_a_later_read(
    reader: PReader,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"foo,bar")
    iterator = reader.segments(path)

    next(iterator)
    append(path, b",baz,")
    consume(iterator)

    assert list(iterator) == []
    assert iterator.state.position == 4


@pytest.mark.parametrize(
    ("start", "index"), [(0, 0), (12, 2)], ids=["file_start", "item_boundary"]
)
def test_align_start_skips_nothing_when_already_aligned(
    reader: PReader, data_file: Path, start: int, index: int
) -> None:
    options = IteratorOptions(start=start)

    aligned = list(reader.segments(data_file, options=options, align_start=True))

    assert aligned == list(reader.segments(data_file, options=options))
    assert aligned == SEGMENTS[index:]


@pytest.mark.parametrize(
    ("start", "remaining", "index"),
    [(12, 2, 4), (14, 3, 5)],
    ids=["aligned", "misaligned"],
)
def test_skip_remaining_counts_down_to_zero(
    reader: PReader, data_file: Path, start: int, remaining: int, index: int
) -> None:
    options = IteratorOptions(start=start, skip=2)
    iterator = reader.segments(data_file, options=options, align_start=True)

    assert iterator.skip_remaining == remaining

    assert next(iterator) == SEGMENTS[index]
    assert iterator.skip_remaining == 0


def test_keep_delimiter_does_not_change_the_position(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    kept = reader.segments(data_file, keep_delimiter=True)
    stripped = reader.segments(data_file, keep_delimiter=False)

    consume(kept)
    consume(stripped)

    assert kept.state.position == stripped.state.position == len(SEGMENT_CONTENT)


def test_end_yields_a_crossing_segment_whole(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(end=15)

    iterator = reader.segments(data_file, options=options)

    assert list(iterator) == SEGMENTS[:3]
    assert iterator.state.position == 3 * (
        len(SEGMENTS[0]) + len(TEST_DEFAULT_DELIMITER)
    )


def test_position_counts_finished_items(reader: PReader, data_file: Path) -> None:
    iterator = reader.segments(data_file)

    next(iterator)

    assert iterator.state.position == 0
    assert iterator.percent() == 0.0

    next(iterator)

    assert iterator.state.position == len(SEGMENTS[0]) + len(TEST_DEFAULT_DELIMITER)


@pytest.mark.parametrize(
    "delimiter",
    [TEST_DEFAULT_DELIMITER, b";", b"|", b"\t", b"\n", b"\x00"],
    ids=["comma", "semicolon", "pipe", "tab", "newline", "null"],
)
def test_splits_utf8_text_between_characters(
    reader: PReader, make_file: Callable[..., Path], delimiter: bytes
) -> None:
    words = [*TEST_UNICODE_TEXT.split(), "鉄"]
    path = make_file(delimiter.join(map(str.encode, words)))

    segments = list(reader.segments(path, delimiter=delimiter))

    assert [segment.decode() for segment in segments] == words


def test_align_start_and_skip_combine(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(start=3, skip=1)

    aligned = list(reader.segments(data_file, options=options, align_start=True))
    unaligned = list(reader.segments(data_file, options=options))

    assert aligned == SEGMENTS[2:]
    assert unaligned == SEGMENTS[1:]


@pytest.mark.parametrize("byte", DELIMITERS)
def test_delimiter_accepts_any_byte(
    reader: PReader, make_file: Callable[..., Path], byte: int
) -> None:
    delimiter, segment = bytes([byte]), bytes([(byte + 1) % 256])
    path = make_file(segment + delimiter + segment)

    assert list(reader.segments(path, delimiter=delimiter)) == [segment, segment]


def test_skip_counts_blank_items_before_skip_empty(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo,,bar,,baz")
    options = IteratorOptions(skip=2)

    first = reader.segments(path, options=options)
    second = reader.segments(path, options=options, skip_empty=True)

    assert list(first) == [b"bar", b"", b"baz"]
    assert list(second) == [b"bar", b"baz"]


def test_file_growth_during_iteration_is_ignored(
    reader: PReader,
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"foo,bar,")
    iterator = reader.segments(path)

    next(iterator)
    append(path, b"more")

    assert list(iterator) == [b"bar"]
    assert iterator.state.position == 8


def test_read_yields_invalid_bytes(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    invalid = b"foo," + TEST_INVALID_UTF8 + b",bar"
    iterator = reader.segments(make_file(invalid))

    assert list(iterator) == [b"foo", TEST_INVALID_UTF8, b"bar"]
    assert iterator.state.position == len(invalid)
    assert iterator.percent() == 100.0


@pytest.mark.parametrize(
    "buffer_capacity", [1, Config().buffer_capacity], ids=["refilled", "default"]
)
def test_growth_holds_back_an_unterminated_segment(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
    buffer_capacity: int,
) -> None:
    reader = make_reader(buffer_capacity=buffer_capacity)
    path = make_file(b"foo,bar")
    iterator = reader.segments(path)

    next(iterator)
    append(path, b"more")

    assert list(iterator) == []
    assert iterator.state.position == 4
    assert iterator.percent() == 400 / 7


def test_raises_when_a_save_fails_on_a_filtered_blank(
    make_reader: Callable[..., PReader], make_file: Callable[..., Path], config: Config
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    reader = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    path = make_file(TEST_DEFAULT_DELIMITER * 2 + b"foo")
    iterator = reader.segments(path, state=TEST_STATE_NAME, skip_empty=True)

    with pytest.raises(FileExistsError):
        next(iterator)

    assert iterator.state.position == 1


def test_raises_when_a_save_fails_while_skipping(
    make_reader: Callable[..., PReader],
    data_file: Path,
    truncate: Callable[[Path, int], None],
    config: Config,
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    reader = make_reader(auto_save_state=True)
    options = IteratorOptions(skip=3)
    iterator = reader.segments(data_file, state=TEST_STATE_NAME, options=options)

    truncate(data_file, 1)

    with pytest.raises(FileExistsError):
        next(iterator)

    assert iterator.state.position == 1


def test_resumed_read_floors_the_last_saved_position(
    reader: PReader,
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
) -> None:
    iterator = reader.segments(data_file, state=TEST_STATE_NAME)
    state = consume(iterator, 3).save()

    saving = make_reader(auto_save_state=True, auto_save_state_bytes=8)
    resumed = saving.segments(data_file, state=state)

    consume(resumed, 2)

    assert saving.states[TEST_STATE_NAME].position == 18


def test_raises_when_the_aligned_file_is_replaced_by_a_directory_without_verification(
    make_reader: Callable[..., PReader], data_file: Path
) -> None:
    reader = make_reader(verify_state=False)
    options = IteratorOptions(start=5)

    state = reader.segments(data_file, state=TEST_STATE_NAME).state

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises((IsADirectoryError, PermissionError), match="os error"):
        reader.segments(data_file, state=state, options=options, align_start=True)


def test_growth_resumes_an_unterminated_segment_whole(
    reader: PReader,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"foo,bar")
    iterator = reader.segments(path, state=TEST_STATE_NAME)

    next(iterator)
    append(path, b"more")
    consume(iterator).save()

    resynced = reader.states.resync(TEST_STATE_NAME, path)

    assert list(reader.segments(path, state=resynced)) == [b"barmore"]


def test_growth_keeps_a_new_segment_apart(
    reader: PReader,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"foo,bar")
    iterator = reader.segments(path, state=TEST_STATE_NAME)

    next(iterator)
    append(path, b",more")
    consume(iterator).save()

    resynced = reader.states.resync(TEST_STATE_NAME, path)

    assert list(reader.segments(path, state=resynced)) == [b"bar", b"more"]


def test_resume_ignores_align_start_and_skip(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    options = IteratorOptions(start=14)

    iterator = reader.segments(
        data_file, state=TEST_STATE_NAME, options=options, align_start=True
    )
    state = consume(iterator, 2).save()

    resumed_options = IteratorOptions(start=0, skip=5)
    resumed = reader.segments(
        data_file, state=state, options=resumed_options, align_start=True
    )

    assert list(resumed) == SEGMENTS[4:]


def test_raises_when_a_save_fails_on_a_skipped_item(
    make_reader: Callable[..., PReader], data_file: Path, config: Config
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    reader = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    options = IteratorOptions(skip=2)
    iterator = reader.segments(data_file, state=TEST_STATE_NAME, options=options)

    with pytest.raises(FileExistsError):
        next(iterator)

    assert iterator.state.position == len(SEGMENTS[0]) + len(TEST_DEFAULT_DELIMITER)


def test_resume_inside_an_item_ignores_align_start_and_skip(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(start=14)

    state = reader.segments(data_file, state=TEST_STATE_NAME, options=options).save()

    resumed_options = IteratorOptions(start=0, skip=5)
    resumed = reader.segments(
        data_file, state=state, options=resumed_options, align_start=True
    )

    assert list(resumed) == [SEGMENTS[2][2:], *SEGMENTS[3:]]


def test_threshold_saves_during_iteration(
    make_reader: Callable[..., PReader], data_file: Path
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=10)
    iterator = reader.segments(data_file, state=TEST_STATE_NAME)

    saved = [
        reader.states[TEST_STATE_NAME].position
        for _ in iterator
        if TEST_STATE_NAME in reader.states
    ]

    assert list(dict.fromkeys(saved)) == [12, 24, 36, 48]
    assert reader.states[TEST_STATE_NAME].position == len(SEGMENT_CONTENT)


def test_resume_applies_a_different_delimiter(
    reader: PReader, make_file: Callable[..., Path], consume: Callable[..., Any]
) -> None:
    content = SEGMENT_CONTENT + b";" + SEGMENT_CONTENT
    path = make_file(content)
    rest_of_first_copy = TEST_DEFAULT_DELIMITER.join(SEGMENTS[1:])

    iterator = reader.segments(path, state=TEST_STATE_NAME)
    state = consume(iterator, 2).save()

    resumed = reader.segments(path, state=state, delimiter=b";")

    assert list(resumed) == [rest_of_first_copy, SEGMENT_CONTENT]


def test_skip_stops_at_a_truncation(
    make_reader: Callable[..., PReader],
    data_file: Path,
    truncate: Callable[[Path, int], None],
) -> None:
    reader = make_reader(verify_state=False)
    reader.bytes(data_file, state=TEST_STATE_NAME).save()

    truncate(data_file, 8)

    options = IteratorOptions(skip=3)
    resumed = reader.segments(data_file, state=TEST_STATE_NAME, options=options)

    assert list(resumed) == []
    assert resumed.state.position == 8
    assert resumed.state.file.size == len(SEGMENT_CONTENT)
