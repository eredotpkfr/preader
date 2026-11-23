import functools
import json
import os

from collections.abc import Callable, Generator, Iterator
from contextlib import AbstractContextManager, contextmanager
from datetime import datetime
from pathlib import Path
from typing import Any

import pytest

from cases.content import LOST_PREFIXES
from common.constants import (
    TEST_LARGE_COPIES,
    TEST_LINE,
    TEST_PRE_EPOCH,
    TEST_STAMP,
    TEST_STATE_NAME,
    TEST_TRACKED_COPIES,
    TEST_TRACKED_NAME,
    TEST_WINDOW,
)
from preader import (
    PReader,
    State,
    StateCorruptError,
    StateMismatchError,
    StateOutdatedError,
    StateRegistry,
)

PAST_WINDOW = TEST_WINDOW + 404
TRACKED_CONTENT = TEST_LINE * TEST_TRACKED_COPIES


@pytest.fixture
def keep_mtime() -> Callable[[Path], AbstractContextManager[None]]:
    @contextmanager
    def _keep_mtime(path: Path) -> Generator[None]:
        stat = path.stat()

        yield

        os.utime(path, ns=(stat.st_atime_ns, stat.st_mtime_ns))

    return _keep_mtime


@pytest.fixture
def overwrite() -> Callable[[Path, int, bytes], None]:
    def _overwrite(path: Path, offset: int, content: bytes) -> None:
        with path.open("r+b") as f:
            f.seek(offset)
            f.write(content)

    return _overwrite


def _leaves(
    payload: dict[str, Any], keys: tuple[str, ...] = ()
) -> Iterator[tuple[str, ...]]:
    for key, value in payload.items():
        if isinstance(value, dict):
            yield from _leaves(value, (*keys, key))
        else:
            yield (*keys, key)


def _changed(value: object) -> object:
    return value + 1 if isinstance(value, int) else f"{value}-changed"


def test_state_fields(reader: PReader, tmp_file: Path) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state

    assert isinstance(state, State)
    assert state.name == TEST_STATE_NAME
    assert state.position == 0
    assert state.file.path == tmp_file
    assert state.timestamps.created_at == state.timestamps.updated_at


def test_state_compares_by_value(
    reader: PReader, tmp_file: Path, tmp_large_file: Path
) -> None:
    iterator = reader.bytes(tmp_file)

    assert iterator.state is not iterator.state
    assert iterator.state == iterator.state
    assert iterator.state != reader.bytes(tmp_large_file).state


def test_state_repr(
    reader: PReader, tmp_file: Path, expected_repr: Callable[..., str]
) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state

    assert repr(state) == expected_repr(
        "State",
        name=state.name,
        file=state.file,
        position=state.position,
        timestamps=state.timestamps,
    )


def test_verify_passes_when_the_file_is_untouched(
    reader: PReader, tmp_large_file: Path
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    reader.states[TEST_STATE_NAME]


@pytest.mark.parametrize("field", ["file", "name", "position", "timestamps"])
def test_state_raises_when_a_field_is_set(
    reader: PReader, tmp_file: Path, field: str
) -> None:
    state = reader.bytes(tmp_file).state

    with pytest.raises(AttributeError, match="is not writable"):
        setattr(state, field, None)


@pytest.mark.usefixtures("requires_non_utf8_names")
def test_bytes_raises_when_the_path_is_not_utf8(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo", name=os.fsdecode(b"data-\xff.bin"))

    with pytest.raises(ValueError, match="invalid UTF-8"):
        reader.bytes(path)


def test_verify_ignores_a_change_past_the_fingerprint_window(
    reader: PReader,
    tmp_large_file: Path,
    overwrite: Callable[[Path, int, bytes], None],
    keep_mtime: Callable[[Path], AbstractContextManager[None]],
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    with keep_mtime(tmp_large_file):
        overwrite(tmp_large_file, PAST_WINDOW, b"bar")

    reader.states[TEST_STATE_NAME]


def test_verify_raises_when_the_file_is_deleted(
    reader: PReader, tmp_large_file: Path
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    tmp_large_file.unlink()

    with pytest.raises(StateOutdatedError, match="file is missing"):
        reader.states[TEST_STATE_NAME]


def test_verify_raises_when_the_file_grew(
    reader: PReader, tmp_large_file: Path, append: Callable[[Path, bytes], None]
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    append(tmp_large_file, b"more")

    with pytest.raises(StateOutdatedError, match="file size changed"):
        reader.states[TEST_STATE_NAME]


def test_resync_raises_when_the_file_is_unreadable(
    reader: PReader, tmp_file: Path, revoke_permissions: Callable[..., Path]
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    revoke_permissions(tmp_file)

    with pytest.raises(PermissionError):
        reader.states.resync(TEST_STATE_NAME, tmp_file)


def test_getitem_raises_when_the_file_is_deleted_before_the_save(
    reader: PReader, tmp_file: Path
) -> None:
    iterator = reader.bytes(tmp_file, state=TEST_STATE_NAME)

    tmp_file.unlink()
    iterator.save()

    with pytest.raises(StateOutdatedError, match="file is missing"):
        reader.states[TEST_STATE_NAME]


def test_verify_raises_when_the_path_is_a_directory(
    reader: PReader, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    tmp_file.unlink()
    tmp_file.mkdir()

    with pytest.raises(IsADirectoryError, match="is a directory"):
        reader.states[TEST_STATE_NAME]


@pytest.mark.usefixtures("requires_pre_epoch_mtime")
def test_verify_raises_when_the_mtime_moves_before_the_epoch(
    reader: PReader, tmp_large_file: Path
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    os.utime(tmp_large_file, (TEST_PRE_EPOCH, TEST_PRE_EPOCH))

    with pytest.raises(StateOutdatedError, match="file mtime changed"):
        reader.states[TEST_STATE_NAME]


def test_verify_raises_when_the_size_changed(
    reader: PReader,
    tmp_large_file: Path,
    truncate: Callable[[Path, int], None],
    keep_mtime: Callable[[Path], AbstractContextManager[None]],
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    with keep_mtime(tmp_large_file):
        truncate(tmp_large_file, PAST_WINDOW)

    with pytest.raises(StateOutdatedError, match="file size changed"):
        reader.states[TEST_STATE_NAME]


def test_verify_raises_when_the_fingerprint_window_changed(
    reader: PReader,
    tmp_large_file: Path,
    overwrite: Callable[[Path, int, bytes], None],
    keep_mtime: Callable[[Path], AbstractContextManager[None]],
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    with keep_mtime(tmp_large_file):
        overwrite(tmp_large_file, 10, b"bar")

    with pytest.raises(StateMismatchError, match="file is not the tracked file"):
        reader.states[TEST_STATE_NAME]


def test_resync_raises_when_the_path_is_a_directory(
    reader: PReader, tmp_file: Path, tmp_path: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    directory = tmp_path / "elsewhere"
    directory.mkdir()

    with pytest.raises(IsADirectoryError, match="is a directory"):
        reader.states.resync(TEST_STATE_NAME, directory)


def test_resync_accepts_anything_for_an_empty_file(
    reader: PReader, empty_file: Path
) -> None:
    reader.bytes(empty_file, state=TEST_STATE_NAME).save()
    empty_file.write_bytes(b"foo")

    resynced = reader.states.resync(TEST_STATE_NAME, empty_file)

    assert resynced.position == 0
    assert resynced.file.size == 3


@pytest.mark.usefixtures("requires_non_utf8_names")
def test_resync_raises_when_the_path_is_not_utf8(
    reader: PReader, tmp_file: Path, make_file: Callable[..., Path]
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    odd = make_file(tmp_file.read_bytes(), name=os.fsdecode(b"data-\xff.bin"))

    with pytest.raises(ValueError, match="invalid UTF-8"):
        reader.states.resync(TEST_STATE_NAME, odd)


@pytest.mark.usefixtures("requires_pre_epoch_mtime")
def test_resync_accepts_an_mtime_before_the_epoch(
    reader: PReader, tmp_large_file: Path
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    os.utime(tmp_large_file, (TEST_PRE_EPOCH, TEST_PRE_EPOCH))

    resynced = reader.states.resync(TEST_STATE_NAME, tmp_large_file)

    assert resynced.file.mtime.timestamp() == TEST_PRE_EPOCH


def test_resync_saves_the_moved_path(
    reader: PReader, tmp_file: Path, make_file: Callable[..., Path]
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    moved = make_file(tmp_file.read_bytes(), name="moved.bin")

    reader.states.resync(TEST_STATE_NAME, moved)

    assert reader.states[TEST_STATE_NAME].file.path == moved


def test_resync_raises_when_the_state_is_tampered(
    reader: PReader,
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["position"] = 2

    with pytest.raises(StateCorruptError, match="modified outside preader"):
        reader.states.resync(TEST_STATE_NAME, tmp_file)


@pytest.mark.parametrize(
    ("name", "directory", "error"),
    [("missing.bin", False, FileNotFoundError), ("elsewhere", True, IsADirectoryError)],
    ids=["missing", "a_directory"],
)
def test_resync_raises_when_the_path_is_not_a_file_without_verification(
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    tmp_path: Path,
    name: str,
    directory: bool,
    error: type[Exception],
) -> None:
    reader = make_reader(verify_state=False)
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    target = tmp_path / name

    if directory:
        target.mkdir()

    with pytest.raises(error):
        reader.states.resync(TEST_STATE_NAME, target)


@pytest.mark.parametrize(("before", "after"), LOST_PREFIXES)
def test_verify_raises_when_the_prefix_is_gone(
    reader: PReader,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    before: bytes,
    after: bytes,
) -> None:
    path = make_file(before, name=TEST_TRACKED_NAME)
    iterator = reader.bytes(path, state=TEST_STATE_NAME)

    consume(iterator, 5).save()

    make_file(after, name=TEST_TRACKED_NAME)

    with pytest.raises(StateMismatchError, match="file is not the tracked file"):
        reader.states[TEST_STATE_NAME]


def test_verify_reports_the_checksum_first(
    reader: PReader,
    tmp_large_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["position"] = 999

    tmp_large_file.unlink()

    with pytest.raises(StateCorruptError, match="modified outside preader"):
        reader.states[TEST_STATE_NAME]


def test_verify_raises_when_the_mtime_changed(
    reader: PReader, tmp_large_file: Path
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()
    original_mtime = tmp_large_file.stat().st_mtime

    os.utime(tmp_large_file, (original_mtime + 3600, original_mtime + 3600))

    with pytest.raises(StateOutdatedError, match="file mtime changed"):
        reader.states[TEST_STATE_NAME]


def test_resync_ignores_a_change_past_the_fingerprint_window(
    reader: PReader,
    tmp_large_file: Path,
    consume: Callable[..., Any],
    overwrite: Callable[[Path, int, bytes], None],
) -> None:
    iterator = reader.bytes(tmp_large_file, state=TEST_STATE_NAME)

    consume(iterator, 6001).save()
    overwrite(tmp_large_file, PAST_WINDOW, b"bar")

    resynced = reader.states.resync(TEST_STATE_NAME, tmp_large_file)

    reader.bytes(tmp_large_file, state=resynced)

    assert resynced.position == 6000


@pytest.mark.parametrize(
    ("content", "position", "expected"),
    [
        pytest.param(TEST_LINE, 8, 200.0, id="position_past_the_size"),
        pytest.param(b"", 5, 500.0, id="zero_size"),
    ],
)
def test_percent_reads_the_stored_numbers(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    content: bytes,
    position: int,
    expected: float,
) -> None:
    reader = make_reader(verify_state=False)
    reader.bytes(make_file(content), state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["position"] = position

    assert reader.states[TEST_STATE_NAME].percent() == expected


@pytest.mark.parametrize(("before", "after"), LOST_PREFIXES)
def test_resync_raises_when_the_prefix_is_gone(
    reader: PReader,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    before: bytes,
    after: bytes,
) -> None:
    path = make_file(before, name=TEST_TRACKED_NAME)
    iterator = reader.bytes(path, state=TEST_STATE_NAME)

    consume(iterator, 5).save()

    make_file(after, name=TEST_TRACKED_NAME)

    with pytest.raises(StateMismatchError, match="file is not the tracked file"):
        reader.states.resync(TEST_STATE_NAME, path)


def test_resync_raises_when_a_tampered_state_tracks_a_changed_file(
    reader: PReader,
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["position"] = 2

    tmp_file.write_bytes(b"bar\n")

    with pytest.raises(StateCorruptError, match="modified outside preader"):
        reader.states.resync(TEST_STATE_NAME, tmp_file)


@pytest.mark.usefixtures("requires_pre_epoch_mtime")
def test_verify_raises_when_an_mtime_before_the_epoch_changed(
    reader: PReader, tmp_large_file: Path
) -> None:
    os.utime(tmp_large_file, (TEST_PRE_EPOCH, TEST_PRE_EPOCH))

    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    os.utime(tmp_large_file, (2 * TEST_PRE_EPOCH, 2 * TEST_PRE_EPOCH))

    with pytest.raises(StateOutdatedError, match="file mtime changed"):
        reader.states[TEST_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
def test_resync_raises_when_the_path_is_a_symlinked_directory(
    reader: PReader, tmp_file: Path, tmp_path: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    directory = tmp_path / "elsewhere"
    directory.mkdir()

    link = tmp_path / "link"
    link.symlink_to(directory, target_is_directory=True)

    with pytest.raises(IsADirectoryError, match="is a directory"):
        reader.states.resync(TEST_STATE_NAME, link)


def test_verify_raises_when_a_tampered_state_tracks_a_changed_file(
    reader: PReader,
    tmp_large_file: Path,
    truncate: Callable[[Path, int], None],
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    keep_mtime: Callable[[Path], AbstractContextManager[None]],
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["position"] = 999

    with keep_mtime(tmp_large_file):
        truncate(tmp_large_file, PAST_WINDOW)

    with pytest.raises(StateCorruptError, match="modified outside preader"):
        reader.states[TEST_STATE_NAME]


def test_resync_keeps_a_position_that_equals_the_size(
    reader: PReader, make_file: Callable[..., Path], consume: Callable[..., Any]
) -> None:
    path = make_file(TRACKED_CONTENT, name=TEST_TRACKED_NAME)
    iterator = reader.bytes(path, state=TEST_STATE_NAME)
    state = consume(iterator).save()

    resynced = reader.states.resync(TEST_STATE_NAME, path)

    assert state.position == 100
    assert resynced.position == 100
    assert resynced.file.size == 100


def test_verify_raises_when_the_mtime_changed_within_a_second(
    reader: PReader, tmp_large_file: Path
) -> None:
    stamp = TEST_STAMP * 1_000_000_000

    os.utime(tmp_large_file, ns=(stamp, stamp))

    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()

    os.utime(tmp_large_file, ns=(stamp + 1_000, stamp + 1_000))

    with pytest.raises(StateOutdatedError, match="file mtime changed"):
        reader.states[TEST_STATE_NAME]


def test_payload_carries_every_field(reader: PReader, tmp_file: Path) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    payload = json.loads(
        reader.states.path(TEST_STATE_NAME).read_text(encoding="utf-8")
    )

    assert sorted(payload) == ["_checksum", "file", "name", "position", "timestamps"]
    assert sorted(payload["file"]) == ["fingerprint", "mtime", "path", "size"]
    assert sorted(payload["timestamps"]) == ["created_at", "updated_at"]


def test_resync_clamps_the_position_to_the_new_size(
    reader: PReader,
    tmp_large_file: Path,
    consume: Callable[..., Any],
    truncate: Callable[[Path, int], None],
) -> None:
    iterator = reader.bytes(tmp_large_file, state=TEST_STATE_NAME)
    state = consume(iterator, 6001).save()

    truncate(tmp_large_file, PAST_WINDOW)

    resynced = reader.states.resync(TEST_STATE_NAME, tmp_large_file)

    reader.bytes(tmp_large_file, state=resynced)

    assert state.position == 6000
    assert resynced.position == PAST_WINDOW
    assert resynced.percent() == 100.0


@pytest.mark.parametrize(
    ("before", "after"),
    [
        (TRACKED_CONTENT, TRACKED_CONTENT + b"more"),
        (TRACKED_CONTENT, TRACKED_CONTENT),
        (TEST_LINE * TEST_LARGE_COPIES, TEST_LINE * TEST_LARGE_COPIES + b"more"),
    ],
    ids=["appended", "unchanged", "grown_past_a_wide_window"],
)
def test_resync_accepts_a_kept_prefix(
    reader: PReader,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    before: bytes,
    after: bytes,
) -> None:
    path = make_file(before, name=TEST_TRACKED_NAME)
    iterator = reader.bytes(path, state=TEST_STATE_NAME)

    consume(iterator, 6).save()

    make_file(after, name=TEST_TRACKED_NAME)

    resynced = reader.states.resync(TEST_STATE_NAME, path)

    reader.bytes(path, state=resynced)

    assert resynced.position == 5
    assert resynced.file.size == len(after)


def test_resync_is_stable_when_repeated(
    reader: PReader, make_file: Callable[..., Path], consume: Callable[..., Any]
) -> None:
    path = make_file(TRACKED_CONTENT, name=TEST_TRACKED_NAME)
    iterator = reader.bytes(path, state=TEST_STATE_NAME)

    consume(iterator, 6).save()

    once = reader.states.resync(TEST_STATE_NAME, path)
    twice = reader.states.resync(TEST_STATE_NAME, path)

    reader.bytes(path, state=twice)

    assert twice.position == 5
    assert twice.file == once.file


def test_resync_moves_the_identity_forward(
    reader: PReader, make_file: Callable[..., Path], consume: Callable[..., Any]
) -> None:
    original = make_file(TRACKED_CONTENT, name=TEST_TRACKED_NAME)
    grown = make_file(TEST_LINE * 30, name="grown.bin")

    iterator = reader.bytes(original, state=TEST_STATE_NAME)

    consume(iterator, 5).save()

    resynced = reader.states.resync(TEST_STATE_NAME, grown)

    assert resynced.file.size == 120

    with pytest.raises(StateMismatchError, match="file is not the tracked file"):
        reader.states.resync(TEST_STATE_NAME, original)


@pytest.mark.parametrize(
    ("before", "after", "read", "position", "percent"),
    [
        (TEST_WINDOW, TEST_WINDOW, 101, 100, 100 * 100 / TEST_WINDOW),
        (TEST_WINDOW + 1, TEST_WINDOW, TEST_WINDOW + 2, TEST_WINDOW, 100.0),
    ],
    ids=["at_the_window", "past_the_window"],
)
def test_resync_accepts_at_the_fingerprint_window(
    reader: PReader,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    before: int,
    after: int,
    read: int,
    position: int,
    percent: float,
) -> None:
    path = make_file(b"a" * before, name=TEST_TRACKED_NAME)
    iterator = reader.bytes(path, state=TEST_STATE_NAME)

    consume(iterator, read).save()

    make_file(b"a" * after, name=TEST_TRACKED_NAME)

    resynced = reader.states.resync(TEST_STATE_NAME, path)

    assert resynced.position == position
    assert resynced.file.size == after
    assert resynced.percent() == percent


def test_resync_updates_the_file_metadata(
    reader: PReader,
    tmp_file: Path,
    make_file: Callable[..., Path],
    fingerprint: Callable[..., str],
    stat_mtime: Callable[[Path], datetime],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    moved = make_file(TEST_LINE + b"bar", name="moved.bin")

    resynced = reader.states.resync(TEST_STATE_NAME, moved)

    assert resynced.file.path == moved
    assert resynced.file.size == len(moved.read_bytes())
    assert resynced.file.mtime == stat_mtime(moved)
    assert resynced.file.fingerprint == fingerprint(moved)


@pytest.mark.parametrize(
    ("after", "position", "percent"),
    [
        (b"bar\n" * TEST_TRACKED_COPIES, 50, 50.0),
        (TEST_LINE * 2, 8, 100.0),
        (b"", 0, 0.0),
    ],
    ids=["replaced", "shrunk", "emptied"],
)
def test_resync_trusts_the_path_without_verification(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    after: bytes,
    position: int,
    percent: float,
) -> None:
    reader = make_reader(verify_state=False)
    path = make_file(TRACKED_CONTENT, name=TEST_TRACKED_NAME)
    iterator = reader.bytes(path, state=TEST_STATE_NAME)

    consume(iterator, 51).save()

    make_file(after, name=TEST_TRACKED_NAME)

    resynced = reader.states.resync(TEST_STATE_NAME, path)

    assert resynced.position == position
    assert resynced.file.size == len(after)
    assert resynced.percent() == percent


def test_checksum_matches_a_known_digest(
    registry: StateRegistry, tmp_file: Path, fingerprint: Callable[..., str]
) -> None:
    payload = {
        "name": TEST_STATE_NAME,
        "file": {
            "path": "/tmp/data.bin",  # noqa: S108
            "size": len(TEST_LINE),
            "mtime": TEST_STAMP * 1_000_000,
            "fingerprint": fingerprint(tmp_file),
        },
        "position": 7,
        "timestamps": {"created_at": TEST_STAMP + 1, "updated_at": TEST_STAMP + 2},
        "_checksum": "0881b148c8b6071356b5cb714b11e46cd2aecc19a316b6498588a736812745dd",
    }
    path = registry.path(TEST_STATE_NAME)

    path.parent.mkdir()
    path.write_text(json.dumps(payload), encoding="utf-8")

    assert registry.resync(TEST_STATE_NAME, tmp_file).position == len(TEST_LINE)


def test_resync_lets_a_grown_file_resume(
    reader: PReader,
    tmp_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    iterator = reader.bytes(tmp_file, state=TEST_STATE_NAME)

    consume(iterator).save()
    append(tmp_file, b"more")

    with pytest.raises(StateOutdatedError, match="file size changed"):
        reader.bytes(tmp_file, state=TEST_STATE_NAME)

    resynced = reader.states.resync(TEST_STATE_NAME, tmp_file)
    resumed = reader.bytes(tmp_file, state=resynced)

    assert resynced.file.size == len(tmp_file.read_bytes())
    assert b"".join(resumed) == b"more"


def test_resync_reseals_a_tampered_state(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    reader = make_reader(verify_state=False)
    path = make_file(b"a" * 20)
    reader.bytes(path, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["position"] = 999

    with pytest.raises(StateCorruptError, match="modified outside preader"):
        make_reader().states[TEST_STATE_NAME]

    resynced = reader.states.resync(TEST_STATE_NAME, path)

    assert resynced.position == 20
    assert make_reader().states[TEST_STATE_NAME] == resynced


def test_resync_lets_a_moved_file_resume(
    reader: PReader, tmp_file: Path, tmp_path: Path, consume: Callable[..., Any]
) -> None:
    content = tmp_file.read_bytes()

    iterator = reader.bytes(tmp_file, state=TEST_STATE_NAME)
    state = consume(iterator, 2).save()

    moved = tmp_file.rename(tmp_path / "moved.bin")

    with pytest.raises(StateOutdatedError, match=r"file path changed .*resync"):
        reader.bytes(moved, state=state)

    resynced = reader.states.resync(TEST_STATE_NAME, moved)
    resumed = reader.bytes(moved, state=resynced)

    assert resumed.state.file.path == moved
    assert b"".join(resumed) == content[1:]


def test_resync_keeps_name_position_and_created_at(
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    reader = make_reader(verify_state=False)
    iterator = reader.bytes(tmp_file, state=TEST_STATE_NAME)

    consume(iterator, 3).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["timestamps"] = {"created_at": 0, "updated_at": 0}

    state = reader.states[TEST_STATE_NAME]

    moved = make_file(tmp_file.read_bytes(), name="moved.bin")

    resynced = reader.states.resync(TEST_STATE_NAME, moved)

    assert resynced.name == state.name
    assert resynced.position == state.position == 2
    assert resynced.timestamps.created_at == state.timestamps.created_at
    assert resynced.timestamps.updated_at > state.timestamps.updated_at


def test_verify_raises_when_any_saved_field_is_tampered(
    reader: PReader, tmp_large_file: Path
) -> None:
    reader.bytes(tmp_large_file, state=TEST_STATE_NAME).save()
    state_path = reader.states.path(TEST_STATE_NAME)
    saved = state_path.read_text(encoding="utf-8")
    leaves = [keys for keys in _leaves(json.loads(saved)) if keys != ("_checksum",)]

    for *parents, key in leaves:
        payload = json.loads(saved)
        container = functools.reduce(dict.__getitem__, parents, payload)
        container[key] = _changed(container[key])

        state_path.write_text(json.dumps(payload), encoding="utf-8")

        expected = "belongs to" if key == "name" else "modified outside preader"

        with pytest.raises(StateCorruptError, match=expected):
            reader.states[TEST_STATE_NAME]

    assert leaves
