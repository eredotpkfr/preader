from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

from common.constants import (
    TEST_INVALID_UTF8,
    TEST_LINE,
    TEST_STATE_NAME,
    TEST_UNICODE_TEXT,
)
from preader import Config, IteratorOptions, PReader

LINES = [f"line-{i}" for i in range(6)]
LINE_CONTENT = "\n".join(LINES).encode()
BLANK_LINE_CONTENT = "\n".join([LINES[0], "", LINES[2], ""]).encode()


@pytest.fixture
def data_file(make_file: Callable[..., Path]) -> Path:
    return make_file(LINE_CONTENT)


@pytest.mark.parametrize(
    ("keepends", "skip_empty"),
    [(True, True), (True, False), (False, True), (False, False)],
    ids=["both", "keepends", "skip_empty", "neither"],
)
def test_line_iterator_attributes(
    reader: PReader, tmp_file: Path, keepends: bool, skip_empty: bool
) -> None:
    options = IteratorOptions(skip=1)

    iterator = reader.lines(
        tmp_file, options=options, keepends=keepends, skip_empty=skip_empty
    )

    assert iterator.keepends is keepends
    assert iterator.skip_empty is skip_empty
    assert iterator.skip_remaining == options.skip


def test_line_iterator_defaults(reader: PReader, tmp_file: Path) -> None:
    iterator = reader.lines(tmp_file)

    assert iterator.keepends is False
    assert iterator.skip_empty is False
    assert iterator.skip_remaining == 0


def test_line_iterator_repr(
    reader: PReader, tmp_file: Path, expected_repr: Callable[..., str]
) -> None:
    iterator = reader.lines(tmp_file, keepends=True, skip_empty=True)

    assert repr(iterator) == expected_repr(
        "LineIterator",
        state=iterator.state,
        keepends=iterator.keepends,
        skip_empty=iterator.skip_empty,
        skip_remaining=iterator.skip_remaining,
    )


def test_splits_on_newline(reader: PReader, make_file: Callable[..., Path]) -> None:
    assert list(reader.lines(make_file(b"foo\nbar\nbaz\n"))) == ["foo", "bar", "baz"]


@pytest.mark.parametrize(
    ("content", "expected"),
    [(b"foo\rbar\rbaz", ["foo\rbar\rbaz"]), (b"foo\rbar\r", ["foo\rbar"])],
    ids=["inside_the_line", "at_the_end"],
)
def test_lone_carriage_return_is_not_a_line_separator(
    reader: PReader, make_file: Callable[..., Path], content: bytes, expected: list[str]
) -> None:
    path = make_file(content)

    assert list(reader.lines(path)) == expected


def test_skip_remaining_is_zero_for_an_empty_file(
    reader: PReader, empty_file: Path
) -> None:
    iterator = reader.lines(empty_file, options=IteratorOptions(skip=2))

    assert iterator.skip_remaining == 0


def test_crlf_strips_both_carriage_return_and_newline(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\r\nbar\r\n")

    assert list(reader.lines(path)) == ["foo", "bar"]


@pytest.mark.parametrize(
    ("keepends", "expected"),
    [(False, ["foo", "bar"]), (True, ["foo\r\r\n", "bar\r\r\r\n"])],
    ids=["stripped", "kept"],
)
def test_repeated_carriage_returns_belong_to_the_terminator(
    reader: PReader, make_file: Callable[..., Path], keepends: bool, expected: list[str]
) -> None:
    path = make_file(b"foo\r\r\nbar\r\r\r\n")

    assert list(reader.lines(path, keepends=keepends)) == expected


def test_mixed_line_endings_are_both_stripped(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\nbar\r\nbaz\n")

    assert list(reader.lines(path)) == ["foo", "bar", "baz"]


def test_skip_empty_skips_an_unterminated_carriage_return(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"line-0\n\r")

    assert list(reader.lines(path, skip_empty=True)) == ["line-0"]


def test_crlf_keepends_preserves_the_full_terminator(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\r\nbar\r\n")

    assert list(reader.lines(path, keepends=True)) == ["foo\r\n", "bar\r\n"]


def test_keepends_preserves_the_newline(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\nbar\n")

    assert list(reader.lines(path, keepends=True)) == ["foo\n", "bar\n"]


def test_keepends_leaves_the_last_line_unterminated(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\nbar")

    assert list(reader.lines(path, keepends=True)) == ["foo\n", "bar"]


def test_skip_empty_skips_crlf_only_lines(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\r\n\r\nbar\r\n")

    assert list(reader.lines(path, skip_empty=True)) == ["foo", "bar"]


def test_position_counts_bytes_not_characters(
    reader: PReader, make_file: Callable[..., Path], consume: Callable[..., Any]
) -> None:
    path = make_file("café\nfoo\n".encode())

    assert consume(reader.lines(path), 2).state.position == 6


def test_align_start_skips_a_partial_line(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(start=9)

    assert list(reader.lines(data_file, options=options, align_start=True)) == LINES[2:]


@pytest.mark.parametrize(
    ("keepends", "skip_empty", "expected"),
    [
        (False, False, ["foo", "", "bar"]),
        (False, True, ["foo", "bar"]),
        (True, False, ["foo\n", "\n", "bar\n"]),
        (True, True, ["foo\n", "bar\n"]),
    ],
    ids=["neither", "skip_empty", "keepends", "both"],
)
def test_blank_lines_follow_keepends_and_skip_empty(
    reader: PReader,
    make_file: Callable[..., Path],
    keepends: bool,
    skip_empty: bool,
    expected: list[str],
) -> None:
    path = make_file(b"foo\n\nbar\n")
    iterator = reader.lines(path, keepends=keepends, skip_empty=skip_empty)

    assert list(iterator) == expected


@pytest.mark.parametrize(
    ("keepends", "skip_empty", "expected"),
    [
        (False, False, ["", "", ""]),
        (False, True, []),
        (True, False, ["\n"] * 3),
        (True, True, []),
    ],
    ids=["neither", "skip_empty", "keepends", "both"],
)
def test_blank_only_file_follows_keepends_and_skip_empty(
    reader: PReader,
    make_file: Callable[..., Path],
    keepends: bool,
    skip_empty: bool,
    expected: list[str],
) -> None:
    path = make_file(b"\n\n\n")
    iterator = reader.lines(path, keepends=keepends, skip_empty=skip_empty)

    assert list(iterator) == expected


def test_align_start_saturates_a_maximum_skip(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(start=2, skip=2**64 - 1)

    assert list(reader.lines(data_file, options=options, align_start=True)) == []


def test_raises_when_a_line_is_not_utf8(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(TEST_LINE + TEST_INVALID_UTF8 + b"\n")

    with pytest.raises(ValueError, match="invalid utf-8"):
        list(reader.lines(path))


def test_skipped_content_is_not_validated(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(TEST_INVALID_UTF8 + b"\nfoo\n")
    options = IteratorOptions(skip=1)

    assert list(reader.lines(path, options=options)) == ["foo"]


def test_align_start_and_skip_count_a_blank_line(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(BLANK_LINE_CONTENT)
    options = IteratorOptions(start=2, skip=1)

    assert next(reader.lines(path, options=options, align_start=True)) == LINES[2]


def test_keeps_a_partial_line_by_default(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(start=9)

    assert list(reader.lines(data_file, options=options)) == [LINES[1][2:], *LINES[2:]]


@pytest.mark.parametrize(
    ("options", "align_start"),
    [(IteratorOptions(skip=1), False), (IteratorOptions(start=1), True)],
    ids=["skip", "align_start"],
)
def test_skipped_items_count_as_finished(
    reader: PReader, data_file: Path, options: IteratorOptions, align_start: bool
) -> None:
    iterator = reader.lines(data_file, options=options, align_start=align_start)

    next(iterator)

    assert iterator.state.position == len(LINES[0]) + 1


def test_reads_multi_byte_characters(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    words = TEST_UNICODE_TEXT.split()
    path = make_file("".join(f"{word}\n" for word in words).encode())

    assert list(reader.lines(path)) == words


@pytest.mark.parametrize(
    ("align_start", "keepends", "skip_empty", "expected"),
    [
        (True, True, True, ["line-2\n"]),
        (True, True, False, ["\n", "line-2\n"]),
        (True, False, True, ["line-2"]),
        (True, False, False, ["", "line-2"]),
        (False, True, True, ["e-0\n", "line-2\n"]),
        (False, True, False, ["e-0\n", "\n", "line-2\n"]),
        (False, False, True, ["e-0", "line-2"]),
        (False, False, False, ["e-0", "", "line-2"]),
    ],
    ids=[
        "aligned_both",
        "aligned_keepends",
        "aligned_skip_empty",
        "aligned_neither",
        "unaligned_both",
        "unaligned_keepends",
        "unaligned_skip_empty",
        "unaligned_neither",
    ],
)
def test_flag_combinations_narrow_the_output(
    reader: PReader,
    make_file: Callable[..., Path],
    align_start: bool,
    keepends: bool,
    skip_empty: bool,
    expected: list[str],
) -> None:
    path = make_file(BLANK_LINE_CONTENT)
    options = IteratorOptions(start=3)

    iterator = reader.lines(
        path,
        options=options,
        keepends=keepends,
        align_start=align_start,
        skip_empty=skip_empty,
    )

    assert list(iterator) == expected


def test_skipping_holds_back_an_unterminated_line(
    reader: PReader,
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"foo\nbar")
    iterator = reader.lines(path, options=IteratorOptions(skip=2))

    append(path, b"more")

    assert list(iterator) == []
    assert iterator.state.position == 4


def test_skipping_past_a_terminated_line_ignores_growth(
    reader: PReader,
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"foo\nbar\n")
    iterator = reader.lines(path, options=IteratorOptions(skip=2))

    append(path, b"more")

    assert list(iterator) == []
    assert iterator.state.position == 8


def test_skipped_blank_line_does_not_consume_the_limit(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(BLANK_LINE_CONTENT)
    options = IteratorOptions(limit=2)

    iterator = reader.lines(path, options=options, skip_empty=True)

    assert list(iterator) == [LINES[0], LINES[2]]


@pytest.mark.parametrize(
    ("start", "index"), [(0, 0), (7, 1)], ids=["file_start", "item_boundary"]
)
def test_align_start_skips_nothing_when_already_aligned(
    reader: PReader, data_file: Path, start: int, index: int
) -> None:
    options = IteratorOptions(start=start)

    aligned = list(reader.lines(data_file, options=options, align_start=True))

    assert aligned == list(reader.lines(data_file, options=options))
    assert aligned == LINES[index:]


@pytest.mark.parametrize(
    ("start", "remaining", "index"),
    [(7, 2, 3), (9, 3, 4)],
    ids=["aligned", "misaligned"],
)
def test_skip_remaining_counts_down_to_zero(
    reader: PReader, data_file: Path, start: int, remaining: int, index: int
) -> None:
    options = IteratorOptions(start=start, skip=2)
    iterator = reader.lines(data_file, options=options, align_start=True)

    assert iterator.skip_remaining == remaining

    assert next(iterator) == LINES[index]
    assert iterator.skip_remaining == 0


def test_keepends_does_not_change_the_position(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    kept = reader.lines(data_file, keepends=True)
    stripped = reader.lines(data_file, keepends=False)

    consume(kept)
    consume(stripped)

    assert kept.state.position == stripped.state.position == len(LINE_CONTENT)


def test_yields_every_ascii_byte_value(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    content = bytes(range(128))
    path = make_file(content)

    iterator = reader.lines(path, keepends=True)

    assert list(iterator) == [content[:11].decode(), content[11:].decode()]


def test_end_yields_a_crossing_line_whole(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(end=17)

    iterator = reader.lines(data_file, options=options)

    assert list(iterator) == LINES[:3]
    assert iterator.state.position == 3 * (len(LINES[0]) + 1)


def test_position_counts_finished_items(reader: PReader, data_file: Path) -> None:
    iterator = reader.lines(data_file)

    next(iterator)

    assert iterator.state.position == 0
    assert iterator.percent() == 0.0

    next(iterator)

    assert iterator.state.position == len(LINES[0]) + 1


def test_align_start_and_skip_combine(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(start=9, skip=1)

    aligned = list(reader.lines(data_file, options=options, align_start=True))
    unaligned = list(reader.lines(data_file, options=options))

    assert aligned == LINES[3:]
    assert unaligned == LINES[2:]


def test_skip_counts_blank_items_before_skip_empty(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\n\nbar\n\nbaz\n")
    options = IteratorOptions(skip=2)

    first = reader.lines(path, options=options)
    second = reader.lines(path, options=options, skip_empty=True)

    assert list(first) == ["bar", "", "baz"]
    assert list(second) == ["bar", "baz"]


def test_file_growth_during_iteration_is_ignored(
    reader: PReader,
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"foo\nbar\n")
    iterator = reader.lines(path)

    next(iterator)
    append(path, b"more")

    assert list(iterator) == ["bar"]
    assert iterator.state.position == 8


def test_raises_when_a_line_is_not_utf8_before_any_save(
    make_reader: Callable[..., PReader], make_file: Callable[..., Path], config: Config
) -> None:
    path = make_file(TEST_LINE + TEST_INVALID_UTF8 + b"\xfe\n")
    reader = make_reader(auto_save_state=True)
    iterator = reader.lines(path, state=TEST_STATE_NAME)

    config.state_dir.write_bytes(b"not a directory")

    assert next(iterator) == "foo"

    with pytest.raises(ValueError, match="invalid utf-8"):
        next(iterator)


@pytest.mark.parametrize(
    "buffer_capacity", [1, Config().buffer_capacity], ids=["refilled", "default"]
)
def test_growth_holds_back_an_unterminated_line(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
    buffer_capacity: int,
) -> None:
    reader = make_reader(buffer_capacity=buffer_capacity)
    path = make_file(b"foo\nbar")
    iterator = reader.lines(path)

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
    path = make_file(b"\n\nfoo\n")
    iterator = reader.lines(path, state=TEST_STATE_NAME, skip_empty=True)

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
    iterator = reader.lines(data_file, state=TEST_STATE_NAME, options=options)

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
    iterator = reader.lines(data_file, state=TEST_STATE_NAME)
    state = consume(iterator, 3).save()

    saving = make_reader(auto_save_state=True, auto_save_state_bytes=10)
    resumed = saving.lines(data_file, state=state)

    consume(resumed, 2)

    assert saving.states[TEST_STATE_NAME].position == 21


def test_resume_applies_a_different_keepends(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\nbar\nbaz\n")
    iterator = reader.lines(path, state=TEST_STATE_NAME)

    assert next(iterator) == "foo"
    assert next(iterator) == "bar"

    state = iterator.save()
    resumed = reader.lines(path, state=state, keepends=True)

    assert list(resumed) == ["bar\n", "baz\n"]


def test_raises_when_the_aligned_file_is_replaced_by_a_directory_without_verification(
    make_reader: Callable[..., PReader], data_file: Path
) -> None:
    reader = make_reader(verify_state=False)
    options = IteratorOptions(start=5)

    state = reader.lines(data_file, state=TEST_STATE_NAME).state

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises((IsADirectoryError, PermissionError), match="os error"):
        reader.lines(data_file, state=state, options=options, align_start=True)


def test_growth_resumes_an_unterminated_line_whole(
    reader: PReader,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"foo\nbar")
    iterator = reader.lines(path, state=TEST_STATE_NAME)

    next(iterator)
    append(path, b"more")
    consume(iterator).save()

    resynced = reader.states.resync(TEST_STATE_NAME, path)

    assert list(reader.lines(path, state=resynced)) == ["barmore"]


def test_growth_keeps_a_new_line_apart(
    reader: PReader,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"foo\nbar")
    iterator = reader.lines(path, state=TEST_STATE_NAME)

    next(iterator)
    append(path, b"\nmore")
    consume(iterator).save()

    resynced = reader.states.resync(TEST_STATE_NAME, path)

    assert list(reader.lines(path, state=resynced)) == ["bar", "more"]


def test_resume_ignores_align_start_and_skip(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    options = IteratorOptions(start=9)

    iterator = reader.lines(
        data_file, state=TEST_STATE_NAME, options=options, align_start=True
    )
    state = consume(iterator, 2).save()

    resumed_options = IteratorOptions(start=0, skip=5)
    resumed = reader.lines(
        data_file, state=state, options=resumed_options, align_start=True
    )

    assert list(resumed) == LINES[3:]


def test_raises_when_a_save_fails_on_a_skipped_item(
    make_reader: Callable[..., PReader], data_file: Path, config: Config
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    reader = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    options = IteratorOptions(skip=2)
    iterator = reader.lines(data_file, state=TEST_STATE_NAME, options=options)

    with pytest.raises(FileExistsError):
        next(iterator)

    assert iterator.state.position == len(LINES[0]) + 1


def test_resume_inside_an_item_ignores_align_start_and_skip(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(start=9)

    state = reader.lines(data_file, state=TEST_STATE_NAME, options=options).save()

    resumed_options = IteratorOptions(start=0, skip=5)
    resumed = reader.lines(
        data_file, state=state, options=resumed_options, align_start=True
    )

    assert list(resumed) == [LINES[1][2:], *LINES[2:]]


def test_threshold_saves_during_iteration(
    make_reader: Callable[..., PReader], data_file: Path
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=10)
    iterator = reader.lines(data_file, state=TEST_STATE_NAME)

    saved = [
        reader.states[TEST_STATE_NAME].position
        for _ in iterator
        if TEST_STATE_NAME in reader.states
    ]

    assert list(dict.fromkeys(saved)) == [14, 28]
    assert reader.states[TEST_STATE_NAME].position == len(LINE_CONTENT)


def test_resume_rereads_an_invalid_line(
    make_reader: Callable[..., PReader], make_file: Callable[..., Path]
) -> None:
    path = make_file(TEST_LINE + TEST_INVALID_UTF8 + b"\nbar\n")
    reader = make_reader(auto_save_state=True)

    with pytest.raises(ValueError, match="invalid utf-8"):
        list(reader.lines(path, state=TEST_STATE_NAME))

    resumed = reader.lines(path, state=TEST_STATE_NAME)

    with pytest.raises(ValueError, match="invalid utf-8"):
        next(resumed)

    assert next(resumed) == "bar"


def test_skip_stops_at_a_truncation(
    make_reader: Callable[..., PReader],
    data_file: Path,
    truncate: Callable[[Path, int], None],
) -> None:
    reader = make_reader(verify_state=False)
    reader.bytes(data_file, state=TEST_STATE_NAME).save()

    truncate(data_file, 8)

    options = IteratorOptions(skip=3)
    resumed = reader.lines(data_file, state=TEST_STATE_NAME, options=options)

    assert list(resumed) == []
    assert resumed.state.position == 8
    assert resumed.state.file.size == len(LINE_CONTENT)


def test_resume_rereads_the_invalid_line_after_a_manual_save(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(TEST_LINE + TEST_INVALID_UTF8 + b"\nbar\n")
    iterator = reader.lines(path, state=TEST_STATE_NAME)

    next(iterator)

    with pytest.raises(ValueError, match="invalid utf-8"):
        next(iterator)

    iterator.save()

    resumed = reader.lines(path, state=TEST_STATE_NAME)

    with pytest.raises(ValueError, match="invalid utf-8"):
        next(resumed)

    assert next(resumed) == "bar"


def test_resume_after_an_invalid_line_stays_aligned(
    make_reader: Callable[..., PReader], make_file: Callable[..., Path]
) -> None:
    path = make_file(TEST_LINE + TEST_INVALID_UTF8 + b"\nbar\nbaz\n")
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    iterator = reader.lines(path, state=TEST_STATE_NAME)

    assert next(iterator) == "foo"

    with pytest.raises(ValueError, match="invalid utf-8"):
        next(iterator)

    assert list(iterator) == ["bar", "baz"]
    assert reader.states[TEST_STATE_NAME].position == 14
    assert list(reader.lines(path, state=TEST_STATE_NAME)) == []


def test_read_counts_the_invalid_line(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(TEST_LINE + TEST_INVALID_UTF8 + b"\nbar\n")
    iterator = reader.lines(path)

    assert next(iterator) == "foo"

    with pytest.raises(ValueError, match="invalid utf-8"):
        next(iterator)

    assert iterator.state.position == 4

    assert next(iterator) == "bar"
    assert iterator.state.position == 6

    with pytest.raises(StopIteration):
        next(iterator)

    assert iterator.state.position == 10
    assert iterator.percent() == 100.0
