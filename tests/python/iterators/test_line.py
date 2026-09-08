import json
import os

from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

from constants import (
    TEST_STATE_NAME,
    TEST_UNSAFE_STATE_NAME_IDS,
    TEST_UNSAFE_STATE_NAMES,
    TEST_WINDOWS_UNSAFE_STATE_NAME_IDS,
    TEST_WINDOWS_UNSAFE_STATE_NAMES,
)
from preader import Config, IteratorOptions, PReader, State, StateError

LINES = [f"line-{i}" for i in range(6)]
LINE_CONTENT = "\n".join(LINES).encode()
BLANK_LINE_CONTENT = "\n".join([LINES[0], "", LINES[2], ""]).encode()


@pytest.fixture
def data_file(make_file: Callable[..., Path]) -> Path:
    return make_file(LINE_CONTENT)


@pytest.mark.parametrize(
    ("options", "expected"),
    [
        (IteratorOptions(start=14), LINES[2:]),
        (IteratorOptions(end=14), LINES[:2]),
        (IteratorOptions(skip=2), LINES[2:]),
        (IteratorOptions(skip=1, limit=2), LINES[1:3]),
    ],
    ids=[
        "start_skips_to_position",
        "end_stops_before_position",
        "skip_skips_items",
        "limit_caps_yielded_items",
    ],
)
def test_options_narrow_the_output(
    reader: PReader, data_file: Path, options: IteratorOptions, expected: object
) -> None:
    assert list(reader.lines(data_file, options=options)) == expected


def test_end_yields_a_crossing_line_whole(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(end=17)
    iterator = reader.lines(data_file, options=options)

    assert list(iterator) == LINES[:3]
    assert iterator.state.position > options.end


def test_skip_stops_at_a_truncation(
    make_reader: Callable[..., PReader],
    data_file: Path,
    truncate: Callable[[Path, int], None],
) -> None:
    reader = make_reader(verify_state=False, auto_load_state=True)
    reader.bytes(data_file, state=TEST_STATE_NAME).state.save()

    truncate(data_file, 8)

    options = IteratorOptions(skip=3)

    assert list(reader.lines(data_file, state=TEST_STATE_NAME, options=options)) == []


def test_skip_past_the_end_yields_nothing(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(skip=1000)

    assert list(reader.lines(data_file, options=options)) == []


def test_align_start_skips_partial_line(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(start=9)

    assert list(reader.lines(data_file, options=options, align_start=True)) == LINES[2:]


def test_align_start_skips_nothing_when_already_aligned(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(start=7)

    aligned = list(reader.lines(data_file, options=options, align_start=True))

    assert aligned == list(reader.lines(data_file, options=options))
    assert aligned == LINES[1:]


def test_keeps_partial_line_by_default(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(start=9)

    assert list(reader.lines(data_file, options=options)) == ["ne-1", *LINES[2:]]


@pytest.mark.parametrize("buffer_capacity", [0, 2], ids=["zero", "tiny"])
def test_buffer_capacity_smaller_than_line_length(
    data_file: Path, make_reader: Callable[..., PReader], buffer_capacity: int
) -> None:
    reader = make_reader(buffer_capacity=buffer_capacity)

    assert list(reader.lines(data_file)) == LINES


def test_raises_when_content_is_invalid_utf8(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\n" + bytes([0xFF, 0xFE]) + b"\n")

    with pytest.raises(OSError, match="valid UTF-8"):
        list(reader.lines(path))


def test_raises_when_skipped_content_is_invalid_utf8(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(bytes([0xFF, 0xFE]) + b"\nfoo\n")
    options = IteratorOptions(skip=1)

    with pytest.raises(OSError, match="valid UTF-8"):
        list(reader.lines(path, options=options))


@pytest.mark.parametrize(
    ("keepends", "expected"),
    [(False, ["foo", "bar"]), (True, ["foo\r\r\n", "bar\r\r\r\n"])],
    ids=["stripped", "keepends"],
)
def test_repeated_carriage_returns_are_stripped_together(
    reader: PReader, make_file: Callable[..., Path], keepends: bool, expected: list[str]
) -> None:
    path = make_file(b"foo\r\r\nbar\r\r\r\n")

    assert list(reader.lines(path, keepends=keepends)) == expected


def test_crlf_strips_both_carriage_return_and_newline(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\r\nbar\r\n")

    assert list(reader.lines(path)) == ["foo", "bar"]


def test_crlf_keepends_preserves_full_terminator(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\r\nbar\r\n")

    assert list(reader.lines(path, keepends=True)) == ["foo\r\n", "bar\r\n"]


def test_lone_carriage_return_is_not_a_line_separator(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\rbar\rbaz")

    assert list(reader.lines(path)) == ["foo\rbar\rbaz"]


def test_resume_with_different_keepends(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\nbar\nbaz\n")
    iterator = reader.lines(path, state=TEST_STATE_NAME)

    assert next(iterator) == "foo"

    state = iterator.state
    state.save()

    resumed = reader.lines(path, state=state, keepends=True)

    assert list(resumed) == ["bar\n", "baz\n"]


def test_resume_applies_skip_when_the_position_equals_the_start(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    saved = consume(reader.lines(data_file, state=TEST_STATE_NAME), 2).state
    saved.save()

    options = IteratorOptions(start=saved.position, skip=1)

    assert list(reader.lines(data_file, state=saved, options=options)) == LINES[3:]


def test_resume_ignores_options_when_already_past_start(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    iterator = reader.lines(data_file, state=TEST_STATE_NAME)

    state = consume(iterator, 3).state
    state.save()

    resumed = reader.lines(
        data_file, state=state, options=IteratorOptions(start=6, skip=1)
    )

    assert list(resumed) == LINES[3:]


def test_end_below_the_position_does_not_rewind_the_state(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(
        auto_save_state=True, auto_save_state_bytes=64, auto_load_state=True
    )

    list(reader.lines(data_file, state=TEST_STATE_NAME))

    saved = reader.states[TEST_STATE_NAME].position

    assert saved == len(data_file.read_bytes())

    list(reader.lines(data_file, state=TEST_STATE_NAME, options=IteratorOptions(end=5)))

    assert reader.states[TEST_STATE_NAME].position == saved


def test_resume_on_fully_consumed_file_yields_nothing(
    reader: PReader, tmp_file: Path, consume: Callable[..., Any]
) -> None:
    iterator = reader.lines(tmp_file, state=TEST_STATE_NAME)

    state = consume(iterator).state
    state.save()

    resumed = reader.lines(tmp_file, state=state)

    assert list(resumed) == []
    assert resumed.percent() == 100.0


def test_threshold_autosave_triggers_mid_iteration(
    data_file: Path, make_reader: Callable[..., PReader], consume: Callable[..., Any]
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=14)
    iterator = reader.lines(data_file, state=TEST_STATE_NAME)

    consume(iterator, 2)

    assert reader.states[TEST_STATE_NAME].position == 14


def test_drop_saves_partial_progress(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True)

    iterator = reader.lines(data_file, state=TEST_STATE_NAME)

    next(iterator)

    del iterator

    assert reader.states[TEST_STATE_NAME].position == 7


def test_drop_does_not_save_when_auto_save_disabled(
    reader: PReader, tmp_file: Path
) -> None:
    iterator = reader.lines(tmp_file, state=TEST_STATE_NAME)

    next(iterator)

    del iterator

    assert TEST_STATE_NAME not in reader.states


def test_zero_threshold_saves_only_at_finalize(
    data_file: Path, make_reader: Callable[..., PReader], consume: Callable[..., Any]
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=0)
    iterator = reader.lines(data_file, state=TEST_STATE_NAME)

    consume(iterator, 3)
    position = iterator.state.position

    assert TEST_STATE_NAME not in reader.states

    del iterator

    assert reader.states[TEST_STATE_NAME].position == position


def test_extra_next_after_exhaustion_does_not_resave(
    data_file: Path, make_reader: Callable[..., PReader], consume: Callable[..., Any]
) -> None:
    reader = make_reader(auto_save_state=True)
    iterator = reader.lines(data_file, state=TEST_STATE_NAME)

    consume(iterator)

    mtime_before = reader.states[TEST_STATE_NAME].path().stat().st_mtime

    with pytest.raises(StopIteration):
        next(iterator)

    assert reader.states[TEST_STATE_NAME].path().stat().st_mtime == mtime_before


def test_autosave_error_propagates_from_unbound_iteration(
    config: Config,
    make_reader: Callable[..., PReader],
    data_file: Path,
    capfd: pytest.CaptureFixture[str],
) -> None:
    config.state_dir.write_bytes(b"foo")

    reader = make_reader(auto_save_state=True, auto_save_state_bytes=5)

    with pytest.raises(StateError, match="io failed"):
        for _ in reader.lines(data_file, state=TEST_STATE_NAME):
            pass

    assert "preader: save failed" not in capfd.readouterr().err


def test_save_error_at_finalize_propagates(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True)

    with pytest.raises(StateError, match="path escapes root"):
        for _ in reader.lines(data_file, state="../../etc/passwd"):
            pass


def test_save_error_propagates_after_a_truncation(
    data_file: Path,
    make_reader: Callable[..., PReader],
    truncate: Callable[[Path, int], None],
) -> None:
    reader = make_reader(auto_save_state=True, buffer_capacity=1)
    iterator = reader.lines(data_file, state="../../etc/passwd")

    next(iterator)

    truncate(data_file, 1)

    with pytest.raises(StateError, match="path escapes root"):
        list(iterator)


def test_save_error_propagates_while_skipping(
    data_file: Path,
    make_reader: Callable[..., PReader],
    truncate: Callable[[Path, int], None],
) -> None:
    reader = make_reader(auto_save_state=True, buffer_capacity=1)
    options = IteratorOptions(skip=3)
    iterator = reader.lines(data_file, state="../../etc/passwd", options=options)

    truncate(data_file, 1)

    with pytest.raises(StateError, match="path escapes root"):
        list(iterator)


def test_save_error_propagates_on_a_skipped_item(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    options = IteratorOptions(skip=2)
    iterator = reader.lines(data_file, state="../../etc/passwd", options=options)

    with pytest.raises(StateError, match="path escapes root"):
        list(iterator)

    assert iterator.state.position == len(LINES[0]) + 1


def test_save_error_propagates_on_a_filtered_blank(
    make_file: Callable[..., Path], make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    path = make_file(b"\nfoo\n")

    iterator = reader.lines(path, state="../../etc/passwd", skip_empty=True)

    with pytest.raises(StateError, match="path escapes root"):
        list(iterator)

    assert iterator.state.position == 1


def test_line_iterator_repr(
    reader: PReader,
    tmp_file: Path,
    reindent: Callable[[str, int], str],
    expected_repr: Callable[..., str],
) -> None:
    iterator = reader.lines(tmp_file, keepends=True, skip_empty=True)

    assert repr(iterator) == expected_repr(
        "LineIterator",
        state=reindent(repr(iterator.state), 2),
        keepends=str(iterator.keepends).lower(),
        skip_empty=str(iterator.skip_empty).lower(),
        skip_remaining=iterator.skip_remaining,
    )


def test_auto_load_state_resumes_previous_position(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_load_state=True)
    iterator = reader.lines(data_file)

    next(iterator)

    iterator.state.save()

    assert reader.lines(data_file).state.position == 7


def test_auto_load_state_ignores_an_unverifiable_state(
    data_file: Path,
    make_reader: Callable[..., PReader],
    append: Callable[[Path, bytes], None],
) -> None:
    reader = make_reader(auto_load_state=True)
    iterator = reader.lines(data_file)

    next(iterator)

    iterator.state.save()

    append(data_file, b"tampered")

    assert reader.lines(data_file).state.position == 0


def test_auto_load_state_resumes_stale_state_without_verification(
    data_file: Path,
    make_reader: Callable[..., PReader],
    append: Callable[[Path, bytes], None],
) -> None:
    reader = make_reader(auto_load_state=True, verify_state=False)
    iterator = reader.lines(data_file)

    next(iterator)

    iterator.state.save()

    append(data_file, b"tampered")

    assert reader.lines(data_file).state.position == 7


def test_auto_load_state_disabled_ignores_existing_state(
    reader: PReader, tmp_file: Path, consume: Callable[..., Any]
) -> None:
    consume(reader.lines(tmp_file)).state.save()

    assert reader.lines(tmp_file).state.position == 0


def test_state_name_change_creates_orphaned_state(
    reader: PReader, tmp_file: Path
) -> None:
    reader.lines(tmp_file, state="job-old").state.save()

    new_state = reader.lines(tmp_file, state="job-new").state

    assert new_state.position == 0
    assert "job-old" in reader.states


@pytest.mark.parametrize("state", [123, [], True], ids=["int", "list", "bool"])
def test_raises_when_state_has_an_unsupported_type(
    reader: PReader, data_file: Path, state: State
) -> None:
    with pytest.raises(TypeError, match="state must be None"):
        reader.lines(data_file, state=state)


@pytest.mark.parametrize(
    ("name", "message"), TEST_UNSAFE_STATE_NAMES.items(), ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_unsafe_name_defers_rejection_to_save(
    reader: PReader, tmp_file: Path, name: str, message: str
) -> None:
    iterator = reader.lines(tmp_file, state=name)
    assert iterator.state.position == 0

    with pytest.raises(StateError, match=message):
        iterator.state.save()


@pytest.mark.skipif(
    os.name != "nt", reason="Windows path syntax is only unsafe on Windows"
)
@pytest.mark.parametrize(
    "name", TEST_WINDOWS_UNSAFE_STATE_NAMES, ids=TEST_WINDOWS_UNSAFE_STATE_NAME_IDS
)
def test_unsafe_windows_name_defers_rejection_to_save(
    reader: PReader, tmp_file: Path, name: str
) -> None:
    iterator = reader.lines(tmp_file, state=name)
    assert iterator.state.position == 0

    with pytest.raises(StateError, match="path escapes root"):
        iterator.state.save()


def test_raises_when_state_object_file_argument_mismatches(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    tracked = make_file(LINE_CONTENT, name="tracked.bin")
    untracked = make_file(LINE_CONTENT, name="untracked.bin")

    state = reader.lines(tracked, state=TEST_STATE_NAME).state
    state.save()

    with pytest.raises(StateError, match=r"file path mismatch .*resync"):
        reader.lines(untracked, state=state)


def test_verify_state_disabled_skips_the_mismatch_check(
    make_file: Callable[..., Path], make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(verify_state=False)
    tracked = make_file(LINE_CONTENT, name="tracked.bin")
    untracked = make_file(LINE_CONTENT.upper(), name="untracked.bin")

    state = reader.lines(tracked, state=TEST_STATE_NAME).state
    state.save()

    resumed = reader.lines(untracked, state=state)

    assert resumed.state.file.path == tracked
    assert list(resumed) == LINES


def test_verify_state_disabled_skips_verification(
    data_file: Path,
    make_reader: Callable[..., PReader],
    append: Callable[[Path, bytes], None],
) -> None:
    reader = make_reader()
    iterator = reader.lines(data_file, state=TEST_STATE_NAME)

    next(iterator)

    state = iterator.state
    state.save()

    append(data_file, b"more")

    lenient_reader = make_reader(verify_state=False)
    resumed = lenient_reader.lines(data_file, state=state)

    assert resumed.state.position == state.position


def test_raises_when_file_deleted_and_verify_disabled(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(verify_state=False)

    state = reader.lines(data_file, state=TEST_STATE_NAME).state
    state.save()

    data_file.unlink()

    with pytest.raises(FileNotFoundError):
        reader.lines(data_file, state=state)


def test_raises_when_the_tracked_file_is_deleted(
    make_file: Callable[..., Path], make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(verify_state=False)
    tracked = make_file(LINE_CONTENT, name="tracked.bin")
    untracked = make_file(LINE_CONTENT, name="untracked.bin")

    state = reader.lines(tracked, state=TEST_STATE_NAME).state

    state.save()
    tracked.unlink()

    with pytest.raises(FileNotFoundError):
        reader.lines(untracked, state=state)


def test_resync_allows_resuming_moved_file(
    reader: PReader, tmp_path: Path, data_file: Path, consume: Callable[..., Any]
) -> None:
    state = consume(reader.lines(data_file, state=TEST_STATE_NAME), 1).state
    state.save()

    moved = data_file.rename(tmp_path / "moved.bin")

    with pytest.raises(StateError, match="resync"):
        reader.lines(moved, state=state)

    resynced = state.resync(moved)
    resumed = reader.lines(moved, state=resynced)

    assert resumed.state.file.path == moved
    assert list(resumed) == LINES[1:]


def test_resync_allows_resuming_grown_file(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    state = consume(reader.lines(data_file, state=TEST_STATE_NAME)).state
    state.save()

    append(data_file, b"more\n")

    with pytest.raises(StateError, match="size mismatch"):
        reader.lines(data_file, state=state)

    resynced = state.resync(data_file)
    resumed = reader.lines(data_file, state=resynced)

    assert resynced.file.size == len(LINE_CONTENT) + len(b"more\n")
    assert list(resumed) == ["more"]


def test_reusing_state_name_for_different_file_reads_fresh_file(
    make_file: Callable[..., Path], make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_load_state=True)

    file_a = make_file(b"foo\n", name="data-1.bin")
    file_b = make_file(b"bar\n", name="data-2.bin")

    reader.lines(file_a, state="shared-name").state.save()

    reused = reader.lines(file_b, state="shared-name")

    assert reused.state.file.path == file_b
    assert next(reused) == "bar"

    reused.state.save()

    assert reader.states["shared-name"].file.path == file_b


def test_auto_name_changes_when_file_moves(
    reader: PReader, tmp_path: Path, data_file: Path
) -> None:
    iterator = reader.lines(data_file)

    next(iterator)

    iterator.state.save()

    moved = data_file.rename(tmp_path / "moved.bin")

    assert reader.lines(moved).state.position == 0


def test_raises_when_resumed_after_file_grows(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    state = consume(reader.lines(data_file, state=TEST_STATE_NAME)).state
    state.save()

    append(data_file, b"more\n")

    with pytest.raises(StateError, match="size mismatch"):
        reader.lines(data_file, state=state)


def test_recorded_file_size_never_refreshes_after_file_grows(
    data_file: Path,
    make_reader: Callable[..., PReader],
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    reader = make_reader(verify_state=False)

    state = consume(reader.lines(data_file, state=TEST_STATE_NAME)).state
    state.save()

    append(data_file, b"more\n")

    options = IteratorOptions(end=100)
    resumed = reader.lines(data_file, state=state, options=options)

    assert list(resumed) == []
    assert resumed.state.file.size == len(LINE_CONTENT)


def test_clear_does_not_affect_live_iterator(reader: PReader, data_file: Path) -> None:
    iterator = reader.lines(data_file, state=TEST_STATE_NAME)

    next(iterator)

    iterator.state.save()
    reader.states.clear()

    assert TEST_STATE_NAME not in reader.states

    next(iterator)

    assert iterator.state.position == 14
    assert reader.lines(data_file, state=TEST_STATE_NAME).state.position == 0


def test_raises_when_reading_a_directory(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(verify_state=False)
    state = reader.lines(data_file, state=TEST_STATE_NAME).state
    state.save()

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises(OSError, match=r"os error"):
        list(reader.lines(data_file, state=state))


def test_raises_when_aligning_on_a_directory(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(verify_state=False)
    state = reader.lines(data_file, state=TEST_STATE_NAME).state
    state.save()

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises(OSError, match=r"os error"):
        reader.lines(
            data_file, state=state, options=IteratorOptions(start=5), align_start=True
        )


def test_raises_when_resumed_file_replaced_by_directory(
    reader: PReader, data_file: Path
) -> None:
    state = reader.lines(data_file, state=TEST_STATE_NAME).state
    state.save()

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises(StateError, match="io failed"):
        reader.lines(data_file, state=state)


@pytest.mark.usefixtures("requires_symlinks")
def test_two_symlinks_to_same_target_share_auto_name(
    reader: PReader, tmp_path: Path, make_file: Callable[..., Path]
) -> None:
    real = make_file(LINE_CONTENT, name="real.bin")

    link1 = tmp_path / "link-1.bin"
    link2 = tmp_path / "link-2.bin"

    link1.symlink_to(real)
    link2.symlink_to(real)

    assert reader.lines(link1).state.name == reader.lines(link2).state.name


def test_state_dir_change_creates_fresh_state(
    data_file: Path, make_reader: Callable[..., PReader], tmp_path: Path
) -> None:
    reader = make_reader()

    iterator = reader.lines(data_file, state=TEST_STATE_NAME)

    next(iterator)

    iterator.state.save()

    other_reader = PReader(config=Config(state_dir=tmp_path / "other-preader"))

    assert other_reader.lines(data_file, state=TEST_STATE_NAME).state.position == 0


def test_state_object_keeps_its_own_state_dir(
    data_file: Path, make_reader: Callable[..., PReader], tmp_path: Path
) -> None:
    owner = make_reader(auto_save_state=True)
    state = owner.bytes(data_file, state=TEST_STATE_NAME).state

    config = Config(
        state_dir=tmp_path / "other-preader", auto_save_state=True, verify_state=False
    )
    reader = PReader(config=config)

    list(reader.lines(data_file, state=state))

    assert (owner.config.state_dir / f"{TEST_STATE_NAME}.state.json").is_file()
    assert not (config.state_dir / f"{TEST_STATE_NAME}.state.json").exists()


def test_resume_ignores_align_start_and_skip(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    options = IteratorOptions(start=9)
    iterator = reader.lines(
        data_file, options=options, align_start=True, state=TEST_STATE_NAME
    )

    state = consume(iterator, 1).state
    state.save()

    resumed_options = IteratorOptions(start=0, skip=5)
    resumed = reader.lines(
        data_file, state=state, options=resumed_options, align_start=True
    )

    assert list(resumed) == LINES[3:]


def test_reads_multi_byte_characters(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file("café\n日本語\n".encode())

    assert list(reader.lines(path)) == ["café", "日本語"]


def test_position_counts_bytes_not_characters(
    reader: PReader, make_file: Callable[..., Path], consume: Callable[..., Any]
) -> None:
    content = "café\n"
    path = make_file(content.encode())
    position = consume(reader.lines(path)).state.position

    assert len(content) == 5
    assert position == 6


def test_mixed_line_endings_are_both_stripped(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\nbar\r\nbaz\n")

    assert list(reader.lines(path)) == ["foo", "bar", "baz"]


def test_blank_only_file_yields_blank_lines(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"\n\n\n")

    assert list(reader.lines(path)) == ["", "", ""]


def test_skip_empty_drops_a_blank_only_file(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"\n\n\n")

    assert list(reader.lines(path, skip_empty=True)) == []


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

    items = reader.lines(
        path,
        options=options,
        align_start=align_start,
        keepends=keepends,
        skip_empty=skip_empty,
    )

    assert list(items) == expected


def test_auto_load_state_ignores_a_corrupt_payload(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_load_state=True)
    iterator = reader.lines(data_file)

    next(iterator)

    state_path = iterator.state.save()
    state_path.write_text("not valid json")

    assert reader.lines(data_file).state.position == 0


def test_auto_load_state_ignores_an_incomplete_payload(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_load_state=True)
    iterator = reader.lines(data_file)

    next(iterator)

    state_path = iterator.state.save()
    payload = json.loads(state_path.read_text())

    del payload["timestamps"]

    state_path.write_text(json.dumps(payload))

    assert reader.lines(data_file).state.position == 0


def test_second_iteration_yields_nothing(reader: PReader, data_file: Path) -> None:
    iterator = reader.lines(data_file)

    list(iterator)

    assert list(iterator) == []


def test_drop_does_not_save_without_progress(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True)
    iterator = reader.lines(data_file, state=TEST_STATE_NAME)

    del iterator

    assert TEST_STATE_NAME not in reader.states


def test_drop_warns_on_stderr_when_saving_fails(
    data_file: Path,
    make_reader: Callable[..., PReader],
    capfd: pytest.CaptureFixture[str],
) -> None:
    reader = make_reader(auto_save_state=True)
    iterator = reader.lines(data_file, state="../../etc/passwd")

    next(iterator)

    del iterator

    assert "preader: save failed" in capfd.readouterr().err


def test_skip_counts_blank_items_before_skip_empty(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\n\nbar\nbaz\n")
    options = IteratorOptions(skip=1)

    assert list(reader.lines(path, options=options)) == ["", "bar", "baz"]
    assert list(reader.lines(path, options=options, skip_empty=True)) == ["bar", "baz"]


def test_skip_beyond_the_item_count_yields_nothing(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(skip=len(LINES) + 1)
    iterator = reader.lines(data_file, options=options)

    assert list(iterator) == []
    assert iterator.state.position == len(LINE_CONTENT)


def test_align_start_and_skip_combine(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(start=9, skip=1)

    aligned = list(reader.lines(data_file, options=options, align_start=True))

    assert aligned == LINES[3:]
    assert list(reader.lines(data_file, options=options)) == LINES[2:]


def test_keepends_leaves_the_last_line_unterminated(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\nbar")

    assert list(reader.lines(path, keepends=True)) == ["foo\n", "bar"]


def test_keepends_does_not_change_the_position(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    kept = consume(reader.lines(data_file, keepends=True)).state.position
    stripped = consume(reader.lines(data_file, keepends=False)).state.position

    assert kept == stripped == len(LINE_CONTENT)


def test_threshold_autosave_records_every_item_boundary(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=10)
    saved: list[int] = []

    for _ in reader.lines(data_file, state=TEST_STATE_NAME):
        if TEST_STATE_NAME not in reader.states:
            continue

        position = reader.states[TEST_STATE_NAME].position

        if not saved or saved[-1] != position:
            saved.append(position)

    assert saved == [14, 28, 41]
    assert reader.states[TEST_STATE_NAME].position == len(LINE_CONTENT)


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


def test_resume_applies_the_limit_again(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_load_state=True)
    options = IteratorOptions(limit=2)
    iterator = reader.lines(data_file, options=options)

    list(iterator)

    iterator.state.save()

    assert len(list(reader.lines(data_file, options=options))) == 2


def test_two_iterators_with_the_same_name_advance_independently(
    reader: PReader, data_file: Path
) -> None:
    first = reader.lines(data_file, state=TEST_STATE_NAME)
    second = reader.lines(data_file, state=TEST_STATE_NAME)

    next(first)

    assert first.state.position == 7
    assert second.state.position == 0


def test_percent_tracks_the_position(reader: PReader, data_file: Path) -> None:
    iterator = reader.lines(data_file)
    size = len(data_file.read_bytes())

    assert iterator.percent() == 0.0

    for _ in iterator:
        assert iterator.percent() == pytest.approx(iterator.state.position / size * 100)

    assert iterator.percent() == 100.0


def test_iteration_survives_the_file_being_deleted(
    make_reader: Callable[..., PReader], data_file: Path
) -> None:
    reader = make_reader(buffer_capacity=1)
    expected = list(reader.lines(data_file))
    iterator = reader.lines(data_file)

    first = next(iterator)
    data_file.unlink()

    assert [first, *iterator] == expected
    assert iterator.state.position == len(LINE_CONTENT)


def test_iteration_stops_at_a_truncation(
    make_reader: Callable[..., PReader],
    data_file: Path,
    truncate: Callable[[Path, int], None],
) -> None:
    reader = make_reader(buffer_capacity=1)
    iterator = reader.lines(data_file)

    next(iterator)

    truncate(data_file, 10)

    list(iterator)

    assert iterator.state.position == 10
