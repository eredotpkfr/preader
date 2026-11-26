import hashlib
import os

from collections.abc import Callable
from contextlib import AbstractContextManager
from pathlib import Path
from typing import Any

import pytest

from cases.iterator import ITERATORS
from cases.name import (
    ALIAS_STATE_NAMES,
    INVALID_STATE_NAME_ERRORS,
    LONE_SURROGATE_STATE_NAMES,
    VALID_STATE_NAMES,
)
from cases.payload import MALFORMED_PAYLOAD_TEXTS, MALFORMED_PAYLOADS
from common.constants import (
    TEST_NESTED_STATE_NAME,
    TEST_OTHER_STATE_NAME,
    TEST_STATE_FILE,
    TEST_STATE_FILE_EXTENSION,
    TEST_STATE_NAME,
    TEST_TRACKED_NAME,
    TEST_UNTRACKED_NAME,
)
from preader import (
    Config,
    IteratorBase,
    IteratorOptions,
    PReader,
    StateConflictError,
    StateCorruptError,
    StateError,
    StateMismatchError,
    StateOutdatedError,
)

pytestmark = pytest.mark.parametrize("make_iterator", ITERATORS)

SHARED_CONTENT = b"".join(b"%d,\n" % index for index in range(4096))  # 23 KiB


@pytest.fixture
def data_file(make_file: Callable[..., Path]) -> Path:
    return make_file(SHARED_CONTENT)


def test_is_an_iterator_base(
    reader: PReader, tmp_file: Path, make_iterator: Callable[..., Any]
) -> None:
    assert isinstance(make_iterator(reader, tmp_file), IteratorBase)


def test_yields_nothing_for_an_empty_file(
    reader: PReader, empty_file: Path, make_iterator: Callable[..., Any]
) -> None:
    assert list(make_iterator(reader, empty_file)) == []


@pytest.mark.parametrize("content", [b"", b"foo"], ids=["empty_file", "unread_file"])
def test_percent_is_zero_before_reading(
    reader: PReader,
    make_file: Callable[..., Path],
    content: bytes,
    make_iterator: Callable[..., Any],
) -> None:
    assert make_iterator(reader, make_file(content)).percent() == 0.0


@pytest.mark.parametrize(
    "options",
    [
        IteratorOptions(start=1000),
        IteratorOptions(end=0),
        IteratorOptions(limit=0),
        IteratorOptions(skip=1000),
        IteratorOptions(start=2**64 - 1, end=2**64 - 1),
    ],
    ids=[
        "start_past_the_end",
        "end_zero",
        "limit_zero",
        "skip_past_the_end",
        "maximum_start_and_end",
    ],
)
def test_boundary_options_yield_nothing(
    reader: PReader,
    tmp_file: Path,
    options: IteratorOptions,
    make_iterator: Callable[..., Any],
) -> None:
    assert list(make_iterator(reader, tmp_file, options=options)) == []


def test_raises_when_the_file_is_missing(
    reader: PReader, tmp_path: Path, make_iterator: Callable[..., Any]
) -> None:
    with pytest.raises(FileNotFoundError):
        make_iterator(reader, tmp_path / "missing.bin")


@pytest.mark.parametrize(
    ("file", "message"),
    [
        (123, r"expected str, bytes or os.PathLike"),
        (None, r"expected str, bytes or os.PathLike"),
        (b"data.bin", "is not an instance of 'str'"),
    ],
    ids=["int", "none", "bytes"],
)
def test_raises_when_the_file_has_an_unsupported_type(
    reader: PReader, file: object, message: str, make_iterator: Callable[..., Any]
) -> None:
    with pytest.raises(TypeError, match=message):
        make_iterator(reader, file)


def test_raises_when_the_state_is_positional(
    reader: PReader, tmp_file: Path, make_iterator: Callable[..., Any]
) -> None:
    with pytest.raises(TypeError, match="positional"):
        make_iterator(reader, tmp_file, TEST_STATE_NAME)


@pytest.mark.parametrize(
    "options",
    [
        IteratorOptions(start=10, end=5),
        IteratorOptions(start=2**64 - 1, end=0),
        IteratorOptions(start=1, end=0),
    ],
    ids=["small", "maximum", "one_apart"],
)
def test_raises_when_the_start_exceeds_the_end(
    reader: PReader,
    tmp_file: Path,
    options: IteratorOptions,
    make_iterator: Callable[..., Any],
) -> None:
    with pytest.raises(ValueError, match="start must not be greater than end"):
        make_iterator(reader, tmp_file, options=options)


@pytest.mark.parametrize(
    "state",
    [123, [], True, Path(TEST_NESTED_STATE_NAME), TEST_NESTED_STATE_NAME.encode()],
    ids=["int", "list", "bool", "path", "bytes"],
)
def test_raises_when_the_state_has_an_unsupported_type(
    reader: PReader, tmp_file: Path, state: object, make_iterator: Callable[..., Any]
) -> None:
    with pytest.raises(TypeError, match="state must be None"):
        make_iterator(reader, tmp_file, state=state)


@pytest.mark.parametrize("options", [None, {}, 5], ids=["none", "dict", "int"])
def test_raises_when_the_options_have_an_unsupported_type(
    reader: PReader, tmp_file: Path, options: object, make_iterator: Callable[..., Any]
) -> None:
    with pytest.raises(TypeError, match="is not an instance of 'IteratorOptions'"):
        make_iterator(reader, tmp_file, options=options)


@pytest.mark.parametrize(("name", "message"), INVALID_STATE_NAME_ERRORS)
def test_raises_when_the_state_name_is_invalid(
    reader: PReader,
    tmp_file: Path,
    name: str,
    message: str,
    make_iterator: Callable[..., Any],
) -> None:
    with pytest.raises(ValueError, match=message):
        make_iterator(reader, tmp_file, state=name)


def test_second_iteration_yields_nothing(
    reader: PReader, data_file: Path, make_iterator: Callable[..., Any]
) -> None:
    iterator = make_iterator(reader, data_file)

    list(iterator)

    assert list(iterator) == []


@pytest.mark.parametrize("name", VALID_STATE_NAMES)
def test_saves_a_state_under_a_valid_name(
    reader: PReader, tmp_file: Path, name: str, make_iterator: Callable[..., Any]
) -> None:
    make_iterator(reader, tmp_file, state=name).save()

    assert list(reader.states.names()) == [name]


@pytest.mark.parametrize(
    "name",
    ["café-日本語.bin", "foo bar; $baz & `qux`.bin"],
    ids=["unicode", "special_characters"],
)
def test_reads_an_unusual_path(
    reader: PReader,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    name: str,
    make_iterator: Callable[..., Any],
) -> None:
    path = make_file(b"foo", name=name)

    assert consume(make_iterator(reader, path)).state.position == len(b"foo")


def test_drop_does_not_save_by_default(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(iterator, 2)

    del iterator

    assert TEST_STATE_NAME not in reader.states


def test_drop_does_not_save_without_progress(
    make_reader: Callable[..., PReader],
    data_file: Path,
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_save_state=True)
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    del iterator

    assert TEST_STATE_NAME not in reader.states


def test_auto_load_state_ignores_an_existing_state_by_default(
    reader: PReader,
    tmp_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, tmp_file)

    consume(iterator).save()

    assert make_iterator(reader, tmp_file).state.position == 0


def test_raises_when_the_range_is_inverted_before_opening_the_file(
    reader: PReader, tmp_path: Path, make_iterator: Callable[..., Any]
) -> None:
    options = IteratorOptions(start=9, end=4)

    with pytest.raises(ValueError, match="start must not be greater than end"):
        make_iterator(reader, tmp_path / "missing.bin", options=options)


def test_treats_an_explicit_none_state_as_auto(
    reader: PReader, tmp_file: Path, make_iterator: Callable[..., Any]
) -> None:
    explicit = make_iterator(reader, tmp_file, state=None)
    implicit = make_iterator(reader, tmp_file)

    assert explicit.state.name == implicit.state.name


@pytest.mark.parametrize("buffer_capacity", [0, 2], ids=["zero", "tiny"])
def test_buffer_capacity_does_not_change_the_output(
    reader: PReader,
    make_reader: Callable[..., PReader],
    data_file: Path,
    buffer_capacity: int,
    make_iterator: Callable[..., Any],
) -> None:
    full = list(make_iterator(reader, data_file))
    small = make_reader(buffer_capacity=buffer_capacity)

    assert list(make_iterator(small, data_file)) == full


def test_yields_one_item_for_a_single_byte_file(
    reader: PReader, make_file: Callable[..., Path], make_iterator: Callable[..., Any]
) -> None:
    path = make_file(b"a")
    expected = "a" if make_iterator is PReader.lines else b"a"

    assert list(make_iterator(reader, path)) == [expected]


def test_save_returns_the_saved_state(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    state = consume(iterator, 2).save()

    assert state == reader.states[TEST_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
def test_follows_a_symlink(
    reader: PReader, tmp_file: Path, make_iterator: Callable[..., Any]
) -> None:
    symlink = tmp_file.parent / "link.bin"
    symlink.symlink_to(tmp_file)

    state = make_iterator(reader, symlink).state

    assert state.file.path == tmp_file


def test_raises_when_the_tracked_file_is_deleted(
    reader: PReader, data_file: Path, make_iterator: Callable[..., Any]
) -> None:
    state = make_iterator(reader, data_file, state=TEST_STATE_NAME).state

    data_file.unlink()

    with pytest.raises(FileNotFoundError):
        make_iterator(reader, data_file, state=state)


def test_state_getter_returns_an_independent_snapshot(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file)
    snapshot = iterator.state

    consume(iterator, 2)

    assert snapshot.position == 0
    assert iterator.state.position > 0


def test_raises_when_a_named_state_tracks_a_grown_file(
    reader: PReader,
    tmp_file: Path,
    append: Callable[[Path, bytes], None],
    make_iterator: Callable[..., Any],
) -> None:
    make_iterator(reader, tmp_file, state=TEST_STATE_NAME).save()

    append(tmp_file, b"more")

    with pytest.raises(StateOutdatedError, match="file size changed"):
        make_iterator(reader, tmp_file, state=TEST_STATE_NAME)


def test_raises_when_the_state_is_unsaved(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file)
    state = consume(iterator, 3).state

    with pytest.raises(StateMismatchError, match="has unsaved progress"):
        make_iterator(reader, data_file, state=state)


def test_auto_load_state_raises_when_the_state_tracks_a_grown_file(
    make_reader: Callable[..., PReader],
    data_file: Path,
    append: Callable[[Path, bytes], None],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_load_state=True)

    make_iterator(reader, data_file).save()

    append(data_file, b"more")

    with pytest.raises(StateOutdatedError, match="file size changed"):
        make_iterator(reader, data_file)


def test_autosave_on_drop_records_a_start_at_a_threshold_multiple(
    make_reader: Callable[..., PReader],
    data_file: Path,
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=4)
    iterator = make_iterator(
        reader, data_file, state=TEST_STATE_NAME, options=IteratorOptions(start=4)
    )

    del iterator

    assert reader.states[TEST_STATE_NAME].position == 4


@pytest.mark.parametrize("payload", MALFORMED_PAYLOAD_TEXTS)
def test_auto_load_state_ignores_a_corrupt_state_by_default(
    reader: PReader, data_file: Path, payload: str, make_iterator: Callable[..., Any]
) -> None:
    saved = make_iterator(reader, data_file).save()

    reader.states.path(saved.name).write_text(payload, encoding="utf-8")

    assert make_iterator(reader, data_file).state.position == 0


def test_resolves_a_relative_path_to_the_same_name(
    reader: PReader,
    tmp_file: Path,
    monkeypatch: pytest.MonkeyPatch,
    make_iterator: Callable[..., Any],
) -> None:
    monkeypatch.chdir(tmp_file.parent)

    relative = make_iterator(reader, tmp_file.name)
    absolute = make_iterator(reader, tmp_file)

    assert relative.state.name == absolute.state.name


def test_save_succeeds_after_the_file_is_deleted(
    reader: PReader,
    tmp_file: Path,
    read_state: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, tmp_file, state=TEST_STATE_NAME)

    tmp_file.unlink()

    saved = iterator.save()
    path = reader.states.path(TEST_STATE_NAME)

    assert read_state(path, saved)


@pytest.mark.parametrize("name", ["foo.bin", "f" * 180 + ".bin"], ids=["short", "long"])
def test_names_the_state_by_the_path_digest(
    reader: PReader,
    make_file: Callable[..., Path],
    name: str,
    make_iterator: Callable[..., Any],
) -> None:
    state = make_iterator(reader, make_file(b"foo", name=name)).state
    digest = hashlib.sha256(os.fsencode(state.file.path)).hexdigest()

    assert state.name == digest


def test_auto_load_state_resumes_previous_position(
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_load_state=True)
    iterator = make_iterator(reader, data_file)
    saved = consume(iterator, 2).save()

    assert make_iterator(reader, data_file).state.position == saved.position


def test_raises_when_a_named_state_tracks_a_moved_file(
    reader: PReader, tmp_file: Path, tmp_path: Path, make_iterator: Callable[..., Any]
) -> None:
    make_iterator(reader, tmp_file, state=TEST_STATE_NAME).save()

    moved = tmp_file.rename(tmp_path / "moved.bin")

    with pytest.raises(StateOutdatedError, match=r"file path changed .*resync"):
        make_iterator(reader, moved, state=TEST_STATE_NAME)


def test_raises_when_the_resumed_file_is_replaced_by_a_directory(
    reader: PReader, data_file: Path, make_iterator: Callable[..., Any]
) -> None:
    state = make_iterator(reader, data_file, state=TEST_STATE_NAME).state

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises(IsADirectoryError, match="is a directory"):
        make_iterator(reader, data_file, state=state)


def test_drop_warns_on_stderr_when_saving_fails(
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    capfd: pytest.CaptureFixture[str],
    config: Config,
    make_iterator: Callable[..., Any],
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    reader = make_reader(auto_save_state=True)
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(iterator, 2)

    del iterator

    assert "preader: save failed" in capfd.readouterr().err


def test_failed_manual_save_keeps_reading(
    reader: PReader, config: Config, tmp_file: Path, make_iterator: Callable[..., Any]
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    iterator = make_iterator(reader, tmp_file)

    with pytest.raises(FileExistsError):
        iterator.save()

    assert next(iterator) == next(make_iterator(reader, tmp_file))


def test_raises_when_the_state_tracks_a_grown_file(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    state = consume(iterator).save()

    append(data_file, b"more")

    with pytest.raises(StateOutdatedError, match="file size changed"):
        make_iterator(reader, data_file, state=state)


def test_two_iterators_with_the_same_name_advance_independently(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    first = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    second = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(first, 2)

    assert first.state.position > 0
    assert second.state.position == 0


@pytest.mark.parametrize(("payload", "message"), MALFORMED_PAYLOADS)
def test_raises_when_a_named_state_is_corrupt(
    reader: PReader,
    tmp_file: Path,
    payload: str,
    message: str,
    make_iterator: Callable[..., Any],
) -> None:
    make_iterator(reader, tmp_file, state=TEST_STATE_NAME).save()

    reader.states.path(TEST_STATE_NAME).write_text(payload, encoding="utf-8")

    with pytest.raises(StateCorruptError, match=message):
        make_iterator(reader, tmp_file, state=TEST_STATE_NAME)


def test_accepts_an_unsaved_state_without_verification(
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)
    iterator = make_iterator(reader, data_file)
    state = consume(iterator, 3).state

    resumed = make_iterator(reader, data_file, state=state)

    assert resumed.state.position == state.position


def test_raises_when_a_named_state_tracks_a_file_with_other_content(
    reader: PReader, make_file: Callable[..., Path], make_iterator: Callable[..., Any]
) -> None:
    tracked = make_file(b"foo", name=TEST_TRACKED_NAME)
    untracked = make_file(b"bar", name=TEST_UNTRACKED_NAME)

    make_iterator(reader, tracked, state=TEST_STATE_NAME).save()

    with pytest.raises(StateMismatchError, match="file is not the tracked file"):
        make_iterator(reader, untracked, state=TEST_STATE_NAME)


def test_raises_when_an_unsaved_state_tracks_a_moved_file(
    reader: PReader,
    data_file: Path,
    tmp_path: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file)
    state = consume(iterator, 3).state

    moved = data_file.rename(tmp_path / "moved.bin")

    with pytest.raises(StateMismatchError, match="has unsaved progress"):
        make_iterator(reader, moved, state=state)


def test_auto_load_state_starts_fresh_when_the_file_moves(
    make_reader: Callable[..., PReader],
    tmp_path: Path,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_load_state=True)
    iterator = make_iterator(reader, data_file)

    consume(iterator, 2).save()

    moved = data_file.rename(tmp_path / "moved.bin")

    assert make_iterator(reader, moved).state.position == 0


def test_resume_on_a_fully_consumed_file_yields_nothing(
    reader: PReader,
    tmp_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, tmp_file, state=TEST_STATE_NAME)
    state = consume(iterator).save()

    resumed = make_iterator(reader, tmp_file, state=state)

    assert list(resumed) == []
    assert resumed.percent() == 100.0


def test_raises_when_the_state_tracks_another_file(
    reader: PReader, make_file: Callable[..., Path], make_iterator: Callable[..., Any]
) -> None:
    tracked = make_file(b"foo", name=TEST_TRACKED_NAME)
    untracked = make_file(b"foo", name=TEST_UNTRACKED_NAME)

    state = make_iterator(reader, tracked, state=TEST_STATE_NAME).state

    with pytest.raises(StateOutdatedError, match=r"file path changed .*resync"):
        make_iterator(reader, untracked, state=state)


def test_raises_when_the_state_tracks_a_file_with_other_content(
    reader: PReader, make_file: Callable[..., Path], make_iterator: Callable[..., Any]
) -> None:
    tracked = make_file(b"foo", name=TEST_TRACKED_NAME)
    untracked = make_file(b"bar", name=TEST_UNTRACKED_NAME)

    state = make_iterator(reader, tracked, state=TEST_STATE_NAME).state

    with pytest.raises(StateMismatchError, match="file is not the tracked file"):
        make_iterator(reader, untracked, state=state)


def test_auto_load_state_resumes_a_stale_state_without_verification(
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_load_state=True, verify_state=False)
    iterator = make_iterator(reader, data_file)
    saved = consume(iterator, 2).save()

    append(data_file, b"more")

    assert make_iterator(reader, data_file).state.position == saved.position


def test_save_overwrites_a_state_it_did_not_load(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    first = make_iterator(reader, data_file)

    consume(first, 3).save()

    second = make_iterator(reader, data_file)
    saved = consume(second, 2).save()

    assert reader.states[saved.name] == saved


def test_save_recreates_a_state_deleted_meanwhile(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(iterator, 3).save()

    del reader.states[TEST_STATE_NAME]

    saved = consume(iterator, 2).save()

    assert reader.states[TEST_STATE_NAME] == saved


def test_save_persists_across_readers(
    reader: PReader,
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, tmp_file, state=TEST_STATE_NAME)
    saved = consume(iterator).save()

    loaded = make_reader().states[TEST_STATE_NAME]

    assert loaded == saved
    assert loaded.position == len(tmp_file.read_bytes())


def test_state_dir_change_creates_a_fresh_state(
    reader: PReader,
    make_reader: Callable[..., PReader],
    tmp_path: Path,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(iterator, 2).save()

    other_reader = make_reader(state_dir=tmp_path / "elsewhere")
    fresh = make_iterator(other_reader, data_file, state=TEST_STATE_NAME)

    assert fresh.state.position == 0


@pytest.mark.usefixtures("requires_symlinks")
def test_raises_when_the_named_state_is_a_dangling_symlink(
    reader: PReader,
    config: Config,
    tmp_path: Path,
    tmp_file: Path,
    make_iterator: Callable[..., Any],
) -> None:
    link = config.state_dir / TEST_STATE_FILE

    config.state_dir.mkdir()
    link.symlink_to(tmp_path / "missing.json")

    with pytest.raises(StateError, match="is an alias of another entry"):
        make_iterator(reader, tmp_file, state=TEST_STATE_NAME)

    assert link.is_symlink()


def test_save_excludes_the_unfinished_item(
    reader: PReader, data_file: Path, make_iterator: Callable[..., Any]
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    next(iterator)

    unfinished = next(iterator)
    state = iterator.save()

    resumed = make_iterator(reader, data_file, state=state)

    assert next(resumed) == unfinished


@pytest.mark.skipif(not hasattr(os, "mkfifo"), reason="a fifo cannot be created here")
def test_raises_when_the_resumed_file_is_replaced_by_a_fifo_without_verification(
    make_reader: Callable[..., PReader],
    data_file: Path,
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)

    make_iterator(reader, data_file, state=TEST_STATE_NAME).save()

    data_file.unlink()
    os.mkfifo(data_file)

    with pytest.raises(OSError, match="not a regular file"):
        make_iterator(reader, data_file, state=TEST_STATE_NAME)


def test_raises_when_the_payload_names_another_state(
    reader: PReader,
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    make_iterator: Callable[..., Any],
) -> None:
    make_iterator(reader, tmp_file, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["name"] = TEST_OTHER_STATE_NAME

    with pytest.raises(StateCorruptError, match="state file belongs to"):
        make_iterator(reader, tmp_file, state=TEST_STATE_NAME)


def test_skip_beyond_the_item_count_yields_nothing(
    reader: PReader, data_file: Path, make_iterator: Callable[..., Any]
) -> None:
    options = IteratorOptions(skip=len(list(make_iterator(reader, data_file))) + 1)

    iterator = make_iterator(reader, data_file, options=options)

    assert list(iterator) == []
    assert iterator.state.position == len(SHARED_CONTENT)


@pytest.mark.parametrize(("payload", "message"), MALFORMED_PAYLOADS)
def test_auto_load_state_raises_when_the_state_is_corrupt(
    make_reader: Callable[..., PReader],
    data_file: Path,
    payload: str,
    message: str,
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_load_state=True)
    saved = make_iterator(reader, data_file).save()

    reader.states.path(saved.name).write_text(payload, encoding="utf-8")

    with pytest.raises(StateCorruptError, match=message):
        make_iterator(reader, data_file)


def test_state_name_change_creates_an_orphaned_state(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    saved = consume(iterator, 2).save()

    new_state = make_iterator(reader, data_file, state=TEST_OTHER_STATE_NAME).state

    assert new_state.position == 0
    assert reader.states[TEST_STATE_NAME] == saved


def test_save_lets_the_last_writer_win_for_a_new_name(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    first = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    second = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(first, 4).save()

    saved = consume(second, 2).save()

    assert reader.states[TEST_STATE_NAME] == saved


def test_default_threshold_saves_only_on_drop(
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_save_state=True)
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    position = consume(iterator, 3).state.position

    assert TEST_STATE_NAME not in reader.states

    del iterator

    assert reader.states[TEST_STATE_NAME].position == position


def test_starts_fresh_after_a_corrupt_state_is_deleted(
    reader: PReader, data_file: Path, make_iterator: Callable[..., Any]
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    iterator.save()

    reader.states.path(TEST_STATE_NAME).write_text("not valid json", encoding="utf-8")

    del reader.states[TEST_STATE_NAME]

    assert make_iterator(reader, data_file, state=TEST_STATE_NAME).state.position == 0


def test_raises_when_a_named_state_tracks_a_touched_file(
    reader: PReader, tmp_file: Path, make_iterator: Callable[..., Any]
) -> None:
    make_iterator(reader, tmp_file, state=TEST_STATE_NAME).save()

    mtime = tmp_file.stat().st_mtime_ns + 3_600_000_000_000

    os.utime(tmp_file, ns=(mtime, mtime))

    with pytest.raises(StateOutdatedError, match="file mtime changed"):
        make_iterator(reader, tmp_file, state=TEST_STATE_NAME)


@pytest.mark.parametrize("name", LONE_SURROGATE_STATE_NAMES)
def test_raises_when_the_state_name_has_a_lone_surrogate(
    reader: PReader, tmp_file: Path, name: str, make_iterator: Callable[..., Any]
) -> None:
    with pytest.raises(TypeError, match="state must be None"):
        make_iterator(reader, tmp_file, state=name)

    make_iterator(reader, tmp_file, state=TEST_STATE_NAME).save()

    assert list(reader.states.names()) == [TEST_STATE_NAME]
    assert len(reader.states) == 1


def test_auto_load_state_raises_when_the_payload_is_incomplete(
    make_reader: Callable[..., PReader],
    data_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_load_state=True)
    saved = make_iterator(reader, data_file).save()

    with tamper(reader.states.path(saved.name)) as payload:
        del payload["timestamps"]

    with pytest.raises(StateCorruptError, match="missing field `timestamps`"):
        make_iterator(reader, data_file)


def test_raises_when_the_state_tracks_another_file_without_verification(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)
    tracked = make_file(b"foo", name=TEST_TRACKED_NAME)
    untracked = make_file(b"bar", name=TEST_UNTRACKED_NAME)

    state = make_iterator(reader, tracked, state=TEST_STATE_NAME).state

    with pytest.raises(StateOutdatedError, match="file path changed"):
        make_iterator(reader, untracked, state=state)


def test_resumes_a_stale_named_state_without_verification(
    make_reader: Callable[..., PReader],
    data_file: Path,
    append: Callable[[Path, bytes], None],
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    saved = consume(iterator, 3).save()

    append(data_file, b"more")

    resumed = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    assert resumed.state.position == saved.position


def test_resumes_a_stale_state_without_verification(
    reader: PReader,
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    state = consume(iterator, 2).save()

    append(data_file, b"more")

    lenient = make_reader(verify_state=False)
    resumed = make_iterator(lenient, data_file, state=state)

    assert resumed.state.position == state.position


@pytest.mark.parametrize(
    "options", [IteratorOptions(), IteratorOptions(skip=1)], ids=["reading", "skipping"]
)
def test_raises_when_the_resumed_file_is_replaced_by_a_directory_without_verification(
    make_reader: Callable[..., PReader],
    data_file: Path,
    options: IteratorOptions,
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)

    state = make_iterator(reader, data_file, state=TEST_STATE_NAME).state

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises((IsADirectoryError, PermissionError), match="os error"):
        list(make_iterator(reader, data_file, state=state, options=options))


@pytest.mark.parametrize(
    "options",
    [IteratorOptions(start=1), IteratorOptions(start=1, skip=1)],
    ids=["start", "start_and_skip"],
)
def test_resume_ignores_options_when_already_past_start(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    options: IteratorOptions,
    make_iterator: Callable[..., Any],
) -> None:
    full = list(make_iterator(reader, data_file))
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    state = consume(iterator, 4).save()

    resumed = make_iterator(reader, data_file, state=state, options=options)

    assert list(resumed) == full[3:]


def test_failed_autosave_exhausts_the_iterator(
    make_reader: Callable[..., PReader],
    config: Config,
    data_file: Path,
    make_iterator: Callable[..., Any],
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    reader = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    next(iterator)

    with pytest.raises(FileExistsError):
        next(iterator)

    with pytest.raises(StopIteration):
        next(iterator)


def test_save_refreshes_the_checksum(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    read_state: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    path = reader.states.path(TEST_STATE_NAME)
    before = read_state(path, iterator.save())["_checksum"]

    after = consume(iterator, 5).save()

    assert read_state(path, after)["_checksum"] != before


@pytest.mark.parametrize(
    ("finished", "limit"),
    [(2, 100), (100, 2)],
    ids=["end_is_tighter", "limit_is_tighter"],
)
def test_tighter_of_end_and_limit_wins(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    finished: int,
    limit: int,
    make_iterator: Callable[..., Any],
) -> None:
    full = list(make_iterator(reader, data_file))
    end = consume(make_iterator(reader, data_file), finished + 1).state.position
    options = IteratorOptions(end=end, limit=limit)

    assert list(make_iterator(reader, data_file, options=options)) == full[:2]


def test_drop_does_not_rewrite_a_state_saved_without_progress(
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_save_state=True)
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(iterator, 3).save()

    saved = reader.states.path(TEST_STATE_NAME).stat().st_ino

    del iterator

    assert reader.states.path(TEST_STATE_NAME).stat().st_ino == saved


def test_raises_when_a_resynced_state_tracks_a_file_with_other_content(
    reader: PReader, make_file: Callable[..., Path], make_iterator: Callable[..., Any]
) -> None:
    tracked = make_file(b"foo", name=TEST_TRACKED_NAME)
    untracked = make_file(b"bar", name=TEST_UNTRACKED_NAME)

    make_iterator(reader, tracked, state=TEST_STATE_NAME).save()

    resynced = reader.states.resync(TEST_STATE_NAME, tracked)

    with pytest.raises(StateMismatchError, match="file is not the tracked file"):
        make_iterator(reader, untracked, state=resynced)


def test_raises_when_a_named_state_was_tampered(
    reader: PReader,
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    make_iterator: Callable[..., Any],
) -> None:
    make_iterator(reader, tmp_file, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["position"] = 999

    with pytest.raises(StateCorruptError, match="modified outside preader") as exc_info:
        make_iterator(reader, tmp_file, state=TEST_STATE_NAME)

    assert exc_info.value.name == TEST_STATE_NAME


def test_iteration_survives_the_file_being_deleted(
    make_reader: Callable[..., PReader],
    data_file: Path,
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(buffer_capacity=1)
    expected = list(make_iterator(reader, data_file))
    iterator = make_iterator(reader, data_file)

    first = next(iterator)

    data_file.unlink()

    assert [first, *iterator] == expected
    assert iterator.state.position == len(SHARED_CONTENT)


def test_raises_when_a_tampered_state_tracks_a_moved_file(
    reader: PReader,
    tmp_file: Path,
    tmp_path: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    make_iterator: Callable[..., Any],
) -> None:
    make_iterator(reader, tmp_file, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["position"] = 1

    moved = tmp_file.rename(tmp_path / "moved.bin")

    with pytest.raises(StateCorruptError, match="modified outside preader"):
        make_iterator(reader, moved, state=TEST_STATE_NAME)


def test_drop_saves_a_state_object_from_another_reader(
    reader: PReader,
    make_reader: Callable[..., PReader],
    data_file: Path,
    tmp_path: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    state = consume(iterator, 2).save()

    elsewhere = make_reader(state_dir=tmp_path / "elsewhere", auto_save_state=True)
    borrowed = make_iterator(elsewhere, data_file, state=state)

    del borrowed

    assert elsewhere.states[TEST_STATE_NAME].position == state.position


@pytest.mark.usefixtures("requires_symlinks")
def test_raises_when_the_state_name_escapes_through_a_symlink(
    reader: PReader,
    config: Config,
    tmp_path: Path,
    tmp_file: Path,
    make_iterator: Callable[..., Any],
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir()

    config.state_dir.mkdir()
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    with pytest.raises(StateError, match="is an alias of another entry"):
        make_iterator(reader, tmp_file, state=f"link/{TEST_STATE_NAME}")

    assert not list(outside.iterdir())


def test_raises_when_the_state_was_loaded_from_a_tampered_file(
    reader: PReader,
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    make_iterator: Callable[..., Any],
) -> None:
    make_iterator(reader, tmp_file, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["position"] = 2

    state = make_reader(verify_state=False).states[TEST_STATE_NAME]

    with pytest.raises(StateMismatchError, match="has unsaved progress"):
        make_iterator(reader, tmp_file, state=state)


def test_save_does_not_disturb_the_iterator(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    full = list(make_iterator(reader, data_file))
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    position = consume(iterator, 6).state.position

    iterator.save()

    assert reader.states[TEST_STATE_NAME].position == position
    assert list(iterator) == full[6:]


def test_raises_when_the_final_save_fails(
    reader: PReader,
    make_reader: Callable[..., PReader],
    config: Config,
    data_file: Path,
    make_iterator: Callable[..., Any],
) -> None:
    full = list(make_iterator(reader, data_file))

    config.state_dir.write_bytes(b"not a directory")

    saving = make_reader(auto_save_state=True)
    iterator = make_iterator(saving, data_file, state=TEST_STATE_NAME)

    assert [next(iterator) for _ in full] == full

    with pytest.raises(FileExistsError):
        next(iterator)


@pytest.mark.parametrize(("name", "alias"), ALIAS_STATE_NAMES)
def test_raises_when_the_state_name_is_a_case_or_unicode_alias(
    reader: PReader,
    config: Config,
    tmp_file: Path,
    name: str,
    alias: str,
    make_iterator: Callable[..., Any],
) -> None:
    make_iterator(reader, tmp_file, state=name).save()

    if not (config.state_dir / f"{alias}{TEST_STATE_FILE_EXTENSION}").exists():
        pytest.skip("the file system tells the two names apart")

    with pytest.raises(StateError, match="is an alias of another entry"):
        make_iterator(reader, tmp_file, state=alias)

    assert name in reader.states.names()


@pytest.mark.usefixtures("requires_symlinks")
def test_symlinks_to_one_target_share_the_autoname(
    reader: PReader,
    tmp_path: Path,
    make_file: Callable[..., Path],
    make_iterator: Callable[..., Any],
) -> None:
    real = make_file(b"foo", name="real.bin")

    link1 = tmp_path / "link-1.bin"
    link2 = tmp_path / "link-2.bin"

    link1.symlink_to(real)
    link2.symlink_to(real)

    first = make_iterator(reader, link1)
    second = make_iterator(reader, link2)

    assert first.state.name == second.state.name


def test_raises_when_a_save_fails_after_a_truncation(
    reader: PReader,
    make_reader: Callable[..., PReader],
    config: Config,
    data_file: Path,
    truncate: Callable[[Path, int], None],
    make_iterator: Callable[..., Any],
) -> None:
    first = next(make_iterator(reader, data_file))

    config.state_dir.write_bytes(b"not a directory")

    saving = make_reader(auto_save_state=True, buffer_capacity=1)
    iterator = make_iterator(saving, data_file, state=TEST_STATE_NAME)

    assert next(iterator) == first

    truncate(data_file, 1)

    with pytest.raises(FileExistsError):
        next(iterator)


def test_drop_saves_a_state_object_at_a_threshold_multiple(
    reader: PReader,
    make_reader: Callable[..., PReader],
    data_file: Path,
    tmp_path: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    state = consume(iterator, 2).save()

    elsewhere = make_reader(
        state_dir=tmp_path / "elsewhere",
        auto_save_state=True,
        auto_save_state_bytes=state.position,
    )
    borrowed = make_iterator(elsewhere, data_file, state=state)

    del borrowed

    assert elsewhere.states[TEST_STATE_NAME].position == state.position


def test_resumes_a_named_state_by_default(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    full = list(make_iterator(reader, data_file))
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    saved = consume(iterator, 3).save()

    resumed = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    assert resumed.state.position == saved.position
    assert list(resumed) == full[2:]


def test_extra_next_after_exhaustion_does_not_resave(
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_save_state=True)
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(iterator)

    saved = reader.states.path(TEST_STATE_NAME).stat().st_ino

    with pytest.raises(StopIteration):
        next(iterator)

    assert reader.states.path(TEST_STATE_NAME).stat().st_ino == saved


def test_threshold_autosave_triggers_mid_iteration(
    reader: PReader,
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    threshold = consume(make_iterator(reader, data_file), 4).state.position
    saving = make_reader(auto_save_state=True, auto_save_state_bytes=threshold)
    iterator = make_iterator(saving, data_file, state=TEST_STATE_NAME)

    consume(iterator, 3)

    assert TEST_STATE_NAME not in saving.states

    next(iterator)

    assert saving.states[TEST_STATE_NAME].position == threshold


def test_resume_applies_the_limit_again(
    reader: PReader,
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    full = list(make_iterator(reader, data_file))
    resuming = make_reader(auto_load_state=True)
    options = IteratorOptions(limit=2)

    iterator = make_iterator(resuming, data_file, options=options)

    consume(iterator).save()

    resumed = make_iterator(resuming, data_file, options=options)

    assert list(resumed) == full[2:4]


def test_save_syncs_the_state_object_with_the_file(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    read_state: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    position = consume(iterator, 13).state.position

    iterator.save()

    state = iterator.state
    payload = read_state(reader.states.path(TEST_STATE_NAME), state)

    assert payload["position"] == position
    assert reader.states[TEST_STATE_NAME] == state


def test_resume_applies_skip_when_the_position_equals_the_start(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    full = list(make_iterator(reader, data_file))
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    saved = consume(iterator, 3).save()

    options = IteratorOptions(start=saved.position, skip=1)
    resumed = make_iterator(reader, data_file, state=saved, options=options)

    assert list(resumed) == full[3:]


def test_drop_does_not_rewrite_a_loaded_state_without_progress(
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_save_state=True)
    first = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(first, 3).save()

    saved = reader.states.path(TEST_STATE_NAME).stat().st_ino
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    del iterator

    assert reader.states.path(TEST_STATE_NAME).stat().st_ino == saved


def test_save_rewinds_to_an_explicit_state_object(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    earlier = consume(iterator, 3).save()

    consume(iterator, 5).save()

    second = make_iterator(reader, data_file, state=earlier)
    rewound = consume(second, 1).save()

    assert reader.states[TEST_STATE_NAME] == rewound
    assert rewound.position == earlier.position


def test_clear_does_not_affect_a_live_iterator(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    saved = consume(iterator, 2).save()

    reader.states.clear()

    assert TEST_STATE_NAME not in reader.states

    next(iterator)

    fresh = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    assert iterator.state.position > saved.position
    assert fresh.state.position == 0


def test_state_object_saves_into_the_readers_state_dir(
    reader: PReader,
    make_reader: Callable[..., PReader],
    data_file: Path,
    tmp_path: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    state = consume(iterator, 2).save()

    elsewhere = make_reader(state_dir=tmp_path / "elsewhere", auto_save_state=True)

    list(make_iterator(elsewhere, data_file, state=state))

    assert reader.states[TEST_STATE_NAME] == state
    assert elsewhere.states[TEST_STATE_NAME].position == len(SHARED_CONTENT)


def test_end_below_the_position_does_not_rewind_the_state(
    make_reader: Callable[..., PReader],
    data_file: Path,
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_save_state=True)

    list(make_iterator(reader, data_file, state=TEST_STATE_NAME))

    saved = reader.states[TEST_STATE_NAME].position

    assert saved == len(SHARED_CONTENT)

    options = IteratorOptions(end=5)

    list(make_iterator(reader, data_file, state=TEST_STATE_NAME, options=options))

    assert reader.states[TEST_STATE_NAME].position == saved


def test_autosave_raises_when_the_state_was_resynced_meanwhile(
    make_reader: Callable[..., PReader],
    data_file: Path,
    tmp_path: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=4)
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(iterator, 10)

    moved = data_file.rename(tmp_path / "moved.bin")
    resynced = reader.states.resync(TEST_STATE_NAME, moved)

    with pytest.raises(StateConflictError, match="saved elsewhere"):
        consume(iterator)

    assert reader.states[TEST_STATE_NAME] == resynced


@pytest.mark.usefixtures("requires_symlinks")
def test_raises_when_the_auto_named_state_is_a_symlink(
    reader: PReader,
    config: Config,
    tmp_path: Path,
    tmp_file: Path,
    make_iterator: Callable[..., Any],
) -> None:
    name = make_iterator(reader, tmp_file).state.name

    outside = tmp_path / "outside.json"
    outside.write_text("{}", encoding="utf-8")

    config.state_dir.mkdir()
    (config.state_dir / f"{name}{TEST_STATE_FILE_EXTENSION}").symlink_to(outside)

    with pytest.raises(StateError, match="is an alias of another entry"):
        make_iterator(reader, tmp_file)

    assert outside.read_text(encoding="utf-8") == "{}"


def test_percent_tracks_the_position(
    reader: PReader, data_file: Path, make_iterator: Callable[..., Any]
) -> None:
    iterator = make_iterator(reader, data_file)
    size = len(SHARED_CONTENT)

    assert iterator.percent() == 0.0

    for _ in iterator:
        assert iterator.percent() == iterator.state.position * 100 / size
        assert iterator.state.percent() == iterator.percent()

    assert iterator.percent() == 100.0
    assert iterator.state.percent() == 100.0


@pytest.mark.usefixtures("requires_symlinks")
def test_save_raises_when_the_state_name_became_an_alias(
    reader: PReader,
    config: Config,
    tmp_path: Path,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir()

    iterator = make_iterator(reader, data_file, state=f"link/{TEST_STATE_NAME}")

    consume(iterator, 2)

    config.state_dir.mkdir()
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    with pytest.raises(StateError, match="is an alias of another entry"):
        iterator.save()

    assert not list(outside.iterdir())


def test_save_raises_when_another_iterator_saved_meanwhile(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    make_iterator(reader, data_file, state=TEST_STATE_NAME).save()

    first = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    second = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    saved = consume(first, 4).save()

    consume(second, 2)

    with pytest.raises(StateConflictError, match="saved elsewhere"):
        second.save()

    assert reader.states[TEST_STATE_NAME] == saved


def test_iteration_stops_at_a_truncation(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    data_file: Path,
    truncate: Callable[[Path, int], None],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(buffer_capacity=1)
    truncated = make_file(SHARED_CONTENT[:4096], name="truncated.bin")
    expected = list(make_iterator(reader, truncated))
    iterator = make_iterator(reader, data_file)

    first = next(iterator)

    truncate(data_file, 4096)

    rest = list(iterator)

    assert [first, *rest] == expected
    assert iterator.state.position == 4096


@pytest.mark.parametrize(
    "verify_state", [True, False], ids=["with_verification", "without_verification"]
)
def test_save_raises_when_the_state_was_resynced_meanwhile(
    make_reader: Callable[..., PReader],
    data_file: Path,
    tmp_path: Path,
    consume: Callable[..., Any],
    verify_state: bool,
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=verify_state)
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(iterator, 3).save()

    moved = data_file.rename(tmp_path / "moved.bin")
    reader.states.resync(TEST_STATE_NAME, moved)

    with pytest.raises(StateConflictError, match="saved elsewhere"):
        iterator.save()

    assert reader.states[TEST_STATE_NAME].file.path == moved


def test_failed_autosave_leaves_the_unyielded_item_unfinished(
    reader: PReader,
    make_reader: Callable[..., PReader],
    config: Config,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    probe = consume(make_iterator(reader, data_file), 2)

    config.state_dir.write_bytes(b"not a directory")

    saving = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    iterator = make_iterator(saving, data_file, state=TEST_STATE_NAME)

    next(iterator)

    with pytest.raises(FileExistsError):
        next(iterator)

    assert list(iterator) == []
    assert iterator.state.position == probe.state.position


@pytest.mark.usefixtures("requires_symlinks")
def test_raises_when_the_state_name_is_a_directory_alias(
    reader: PReader, config: Config, tmp_file: Path, make_iterator: Callable[..., Any]
) -> None:
    make_iterator(reader, tmp_file, state=f"real/{TEST_STATE_NAME}").save()

    saved = reader.states.path(f"real/{TEST_STATE_NAME}")
    before = saved.read_bytes()

    (config.state_dir / "alias").symlink_to(saved.parent, target_is_directory=True)

    with pytest.raises(StateError, match="is an alias of another entry"):
        make_iterator(reader, tmp_file, state=f"alias/{TEST_STATE_NAME}")

    assert saved.read_bytes() == before


@pytest.mark.usefixtures("requires_readable_directories")
def test_ends_after_an_os_error_that_reads_nothing(
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)
    state = consume(make_iterator(reader, data_file, state=TEST_STATE_NAME), 2).save()

    data_file.unlink()
    data_file.mkdir()

    iterator = make_iterator(reader, data_file, state=state)

    with pytest.raises(IsADirectoryError, match="os error"):
        next(iterator)

    assert list(iterator) == []
    assert iterator.state.position == state.position


def test_recorded_file_size_never_refreshes_after_the_file_grows(
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    state = consume(iterator).state

    append(data_file, b"more")

    options = IteratorOptions(end=2 * len(SHARED_CONTENT))
    resumed = make_iterator(reader, data_file, state=state, options=options)

    assert list(resumed) == []
    assert resumed.state.file.size == len(SHARED_CONTENT)


@pytest.mark.parametrize(
    ("fields", "expected"),
    [
        ({"start": 5}, slice(5, None)),
        ({"end": 5}, slice(5)),
        ({"skip": 3}, slice(3, None)),
        ({"start": 5, "skip": 3}, slice(8, None)),
        ({"start": 10, "limit": 3}, slice(10, 13)),
        ({"start": 5, "end": 10}, slice(5, 10)),
        ({"start": 5, "skip": 3, "limit": 4}, slice(8, 12)),
        ({"skip": 1, "limit": 2}, slice(1, 3)),
    ],
    ids=[
        "start",
        "end",
        "skip",
        "start_and_skip",
        "start_and_limit",
        "start_and_end",
        "start_skip_and_limit",
        "skip_and_limit",
    ],
)
def test_options_narrow_the_output(
    reader: PReader,
    data_file: Path,
    fields: dict[str, int],
    expected: slice,
    make_iterator: Callable[..., Any],
) -> None:
    plain = make_iterator(reader, data_file)
    starts = [plain.state.position for _ in plain]
    full = list(make_iterator(reader, data_file))
    options = IteratorOptions(
        **{
            key: starts[value] if key in {"start", "end"} else value
            for key, value in fields.items()
        }
    )

    assert list(make_iterator(reader, data_file, options=options)) == full[expected]


@pytest.mark.usefixtures("requires_symlinks")
def test_save_raises_when_a_saved_state_name_became_an_alias(
    reader: PReader,
    config: Config,
    tmp_path: Path,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir()

    iterator = make_iterator(reader, data_file, state=f"link/{TEST_STATE_NAME}")

    consume(iterator, 2).save()

    (config.state_dir / "link").rename(tmp_path / "moved")
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    with pytest.raises(StateError, match="is an alias of another entry"):
        iterator.save()

    assert not list(outside.iterdir())


def test_raises_when_a_threshold_save_fails(
    reader: PReader,
    make_reader: Callable[..., PReader],
    config: Config,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    full = list(make_iterator(reader, data_file))
    threshold = consume(make_iterator(reader, data_file), 3).state.position

    config.state_dir.write_bytes(b"not a directory")

    saving = make_reader(auto_save_state=True, auto_save_state_bytes=threshold)
    iterator = make_iterator(saving, data_file, state=TEST_STATE_NAME)

    assert [next(iterator) for _ in range(2)] == full[:2]

    with pytest.raises(FileExistsError):
        next(iterator)


def test_next_keeps_the_saved_timestamps(
    make_reader: Callable[..., PReader],
    data_file: Path,
    consume: Callable[..., Any],
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)

    make_iterator(reader, data_file, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["timestamps"] = {"created_at": 0, "updated_at": 0}

    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    saved = iterator.state

    consume(iterator, 3)

    assert iterator.state.position > saved.position
    assert iterator.state.timestamps == saved.timestamps


def test_failed_autosave_while_skipping_keeps_the_skipped_items_finished(
    reader: PReader,
    make_reader: Callable[..., PReader],
    config: Config,
    data_file: Path,
    make_iterator: Callable[..., Any],
) -> None:
    options = IteratorOptions(skip=2)
    probe = make_iterator(reader, data_file, options=options)

    next(probe)

    config.state_dir.write_bytes(b"not a directory")

    saving = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    iterator = make_iterator(saving, data_file, state=TEST_STATE_NAME, options=options)

    with pytest.raises(FileExistsError):
        next(iterator)

    assert list(iterator) == []
    assert iterator.state.position == probe.state.position


def test_drop_keeps_a_state_resynced_meanwhile(
    make_reader: Callable[..., PReader],
    data_file: Path,
    tmp_path: Path,
    consume: Callable[..., Any],
    capfd: pytest.CaptureFixture[str],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(auto_save_state=True)
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    saved = consume(iterator, 3).save()

    moved = data_file.rename(tmp_path / "moved.bin")
    reader.states.resync(TEST_STATE_NAME, moved)

    next(iterator)

    del iterator

    stderr = capfd.readouterr().err
    resumed = make_iterator(reader, moved, state=TEST_STATE_NAME)

    assert f"save failed: state '{TEST_STATE_NAME}': state file was saved" in stderr
    assert resumed.state.position == saved.position


def test_save_leaves_the_state_untouched_when_it_fails(
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)

    make_iterator(reader, tmp_file, state=TEST_STATE_NAME).save()

    state_path = reader.states.path(TEST_STATE_NAME)

    with tamper(state_path) as payload:
        payload["timestamps"] = {"created_at": 0, "updated_at": 0}

    iterator = make_iterator(reader, tmp_file, state=TEST_STATE_NAME)
    before = iterator.state

    state_path.unlink()
    state_path.mkdir()

    with pytest.raises((IsADirectoryError, PermissionError), match="os error"):
        iterator.save()

    assert iterator.state == before


def test_save_keeps_raising_after_the_state_was_resynced(
    reader: PReader,
    data_file: Path,
    tmp_path: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    saved = consume(iterator, 3).save()

    moved = data_file.rename(tmp_path / "moved.bin")
    reader.states.resync(TEST_STATE_NAME, moved)

    with pytest.raises(StateConflictError, match="saved elsewhere"):
        iterator.save()

    next(iterator)

    with pytest.raises(StateConflictError, match="saved elsewhere"):
        iterator.save()

    resumed = make_iterator(reader, moved, state=TEST_STATE_NAME)

    assert resumed.state.position == saved.position


def test_save_updates_only_the_updated_at(
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    read_state: Callable[..., Any],
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    make_iterator: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)

    make_iterator(reader, tmp_file, state=TEST_STATE_NAME).save()

    state_path = reader.states.path(TEST_STATE_NAME)

    with tamper(state_path) as payload:
        payload["timestamps"] = {"created_at": 0, "updated_at": 0}

    iterator = make_iterator(reader, tmp_file, state=TEST_STATE_NAME)
    created_at = iterator.state.timestamps.created_at

    saved = iterator.save()

    assert saved.timestamps.created_at == created_at
    assert saved.timestamps.updated_at > created_at
    assert read_state(state_path, saved)


def test_resync_lets_a_moved_file_resume(
    reader: PReader,
    tmp_path: Path,
    data_file: Path,
    consume: Callable[..., Any],
    make_iterator: Callable[..., Any],
) -> None:
    full = list(make_iterator(reader, data_file))
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)
    state = consume(iterator, 2).save()

    moved = data_file.rename(tmp_path / "moved.bin")

    with pytest.raises(StateOutdatedError, match=r"file path changed .*resync"):
        make_iterator(reader, moved, state=state)

    resynced = reader.states.resync(TEST_STATE_NAME, moved)
    resumed = make_iterator(reader, moved, state=resynced)

    assert resumed.state.file.path == moved
    assert list(resumed) == full[1:]


def test_resync_lets_a_grown_file_resume(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
    make_iterator: Callable[..., Any],
) -> None:
    iterator = make_iterator(reader, data_file, state=TEST_STATE_NAME)

    consume(iterator).save()

    append(data_file, b"more")

    with pytest.raises(StateOutdatedError, match="file size changed"):
        make_iterator(reader, data_file, state=TEST_STATE_NAME)

    options = IteratorOptions(start=len(SHARED_CONTENT))
    appended = list(make_iterator(reader, data_file, options=options))
    resynced = reader.states.resync(TEST_STATE_NAME, data_file)
    resumed = make_iterator(reader, data_file, state=resynced)

    assert resynced.file.size == len(SHARED_CONTENT) + len(b"more")
    assert list(resumed) == appended
