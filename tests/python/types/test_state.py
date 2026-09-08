import json
import os
import re

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
from preader import Config, PReader, State, StateError


def test_state_fields(config: Config, reader: PReader, tmp_file: Path) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state

    assert state.name == TEST_STATE_NAME
    assert state.position == 0
    assert state.file.path == tmp_file
    assert state.path() == config.state_dir / f"{state.name}.state.json"
    assert state.timestamps.created_at == state.timestamps.updated_at


@pytest.mark.parametrize(
    ("name", "message"), TEST_UNSAFE_STATE_NAMES.items(), ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_path_raises_when_name_is_unsafe(
    reader: PReader, tmp_file: Path, name: str, message: str
) -> None:
    state = reader.bytes(tmp_file, state=name).state

    with pytest.raises(StateError, match=message):
        state.path()


@pytest.mark.skipif(
    os.name != "nt", reason="Windows path syntax is only unsafe on Windows"
)
@pytest.mark.parametrize(
    "name", TEST_WINDOWS_UNSAFE_STATE_NAMES, ids=TEST_WINDOWS_UNSAFE_STATE_NAME_IDS
)
def test_path_raises_when_a_windows_name_is_unsafe(
    reader: PReader, tmp_file: Path, name: str
) -> None:
    state = reader.bytes(tmp_file, state=name).state

    with pytest.raises(StateError, match="path escapes root"):
        state.path()


def test_checksum_is_stable(reader: PReader, tmp_file: Path) -> None:
    state = reader.bytes(tmp_file).state
    checksum = state.checksum()

    assert re.fullmatch(r"[0-9a-f]{64}", checksum)
    assert state.checksum() == checksum


@pytest.mark.parametrize("content", [b"", b"foo"], ids=["empty_file", "unread_file"])
def test_percent_before_reading(
    reader: PReader, make_file: Callable[..., Path], content: bytes
) -> None:
    assert reader.bytes(make_file(content)).percent() == 0.0


def test_bytes_read_match_file_content(reader: PReader, tmp_file: Path) -> None:
    content = tmp_file.read_bytes()
    read_bytes = list(reader.bytes(tmp_file))

    assert all(len(chunk) == 1 for chunk in read_bytes)
    assert b"".join(read_bytes) == content


def test_save_returns_created_path(
    reader: PReader, tmp_file: Path, read_state: Callable[..., Any]
) -> None:
    state = reader.bytes(tmp_file).state
    assert not state.path().exists()

    assert state.save() == state.path()
    assert state.path().exists()
    assert read_state(state)


@pytest.mark.repeat(10)
def test_save_updates_timestamp_but_not_created_at(
    reader: PReader, tmp_file: Path, read_state: Callable[..., Any]
) -> None:
    state = reader.bytes(tmp_file).state
    created_at = state.timestamps.created_at

    state.save()
    state.save()

    assert state.timestamps.created_at == created_at
    assert state.timestamps.updated_at > created_at
    assert read_state(state)


def test_save_persists_across_new_reader(
    make_reader: Callable[..., PReader],
    reader: PReader,
    tmp_file: Path,
    consume: Callable[..., Any],
) -> None:
    iterator = consume(reader.bytes(tmp_file, state=TEST_STATE_NAME))

    iterator.state.save()

    new_reader = make_reader()
    loaded = new_reader.states[TEST_STATE_NAME]

    assert loaded.position == len(tmp_file.read_bytes())


def test_save_raises_when_state_dir_blocked(
    config: Config, make_reader: Callable[..., PReader], tmp_file: Path
) -> None:
    config.state_dir.write_bytes(b"foo")

    state = make_reader().bytes(tmp_file).state

    with pytest.raises(StateError, match="AlreadyExists"):
        state.save()


def test_save_raises_when_state_path_is_a_directory(
    reader: PReader, tmp_file: Path
) -> None:
    state = reader.bytes(tmp_file).state
    state.path().mkdir(parents=True)

    with pytest.raises(StateError, match="io failed"):
        state.save()


def _get_unverified_state(make_reader: Callable[..., PReader], name: str) -> State:
    return make_reader(verify_state=False).states[name]


def _new_unverified_state(
    make_reader: Callable[..., PReader], data_file: Path
) -> State:
    reader = make_reader(verify_state=False)

    reader.bytes(data_file, state=TEST_STATE_NAME).state.save()

    return _get_unverified_state(make_reader, TEST_STATE_NAME)


def test_verify_passes_when_untouched(
    make_reader: Callable[..., PReader], tmp_large_file: Path
) -> None:
    _new_unverified_state(make_reader, tmp_large_file).verify()


def test_verify_raises_when_checksum_mismatch(
    make_reader: Callable[..., PReader], tmp_large_file: Path
) -> None:
    state = _new_unverified_state(make_reader, tmp_large_file)
    payload = json.loads(state.path().read_text())

    payload.update(position=999)
    state.path().write_text(json.dumps(payload))

    tampered = _get_unverified_state(make_reader, state.name)

    with pytest.raises(StateError, match="checksum mismatch"):
        tampered.verify()


def test_verify_raises_when_size_mismatch(
    make_reader: Callable[..., PReader],
    tmp_large_file: Path,
    truncate: Callable[[Path, int], None],
) -> None:
    state = _new_unverified_state(make_reader, tmp_large_file)
    original_mtime = tmp_large_file.stat().st_mtime

    truncate(tmp_large_file, 4500)

    os.utime(tmp_large_file, (original_mtime, original_mtime))

    with pytest.raises(StateError, match="file size mismatch"):
        state.verify()


@pytest.mark.usefixtures("requires_pre_epoch_mtime")
def test_verify_raises_when_mtime_precedes_the_epoch(
    make_reader: Callable[..., PReader], tmp_large_file: Path
) -> None:
    state = _new_unverified_state(make_reader, tmp_large_file)

    os.utime(tmp_large_file, (-86400, -86400))

    with pytest.raises(StateError, match="io failed"):
        state.verify()


def test_verify_raises_when_mtime_mismatch(
    make_reader: Callable[..., PReader], tmp_large_file: Path
) -> None:
    state = _new_unverified_state(make_reader, tmp_large_file)
    original_mtime = tmp_large_file.stat().st_mtime

    os.utime(tmp_large_file, (original_mtime + 3600, original_mtime + 3600))

    with pytest.raises(StateError, match="file mtime mismatch"):
        state.verify()


def test_verify_raises_when_fingerprint_mismatch(
    make_reader: Callable[..., PReader],
    tmp_large_file: Path,
    overwrite: Callable[[Path, int, bytes], None],
) -> None:
    state = _new_unverified_state(make_reader, tmp_large_file)
    original_mtime = tmp_large_file.stat().st_mtime

    overwrite(tmp_large_file, 10, b"\xff")

    os.utime(tmp_large_file, (original_mtime, original_mtime))

    with pytest.raises(StateError, match="file fingerprint mismatch"):
        state.verify()


def test_verify_blind_spot_beyond_4096_bytes(
    make_reader: Callable[..., PReader],
    tmp_large_file: Path,
    overwrite: Callable[[Path, int, bytes], None],
) -> None:
    state = _new_unverified_state(make_reader, tmp_large_file)
    original_mtime = tmp_large_file.stat().st_mtime

    overwrite(tmp_large_file, 4500, b"\xff")

    os.utime(tmp_large_file, (original_mtime, original_mtime))

    state.verify()


def test_verify_raises_when_file_deleted(
    make_reader: Callable[..., PReader], tmp_large_file: Path
) -> None:
    state = _new_unverified_state(make_reader, tmp_large_file)

    tmp_large_file.unlink()

    with pytest.raises(StateError, match="NotFound") as exc_info:
        state.verify()

    assert "mismatch" not in str(exc_info.value)


def test_resync_updates_file_metadata(
    reader: PReader, tmp_file: Path, tmp_path: Path, fingerprint: Callable[..., str]
) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    state.save()

    moved = tmp_path / "moved.bin"
    moved.write_bytes(tmp_file.read_bytes())

    resynced = state.resync(moved)

    assert resynced.file.path == moved
    assert resynced.file.size == len(moved.read_bytes())
    assert resynced.file.mtime.timestamp() == int(moved.stat().st_mtime)
    assert resynced.file.fingerprint == fingerprint(moved)


def test_resync_preserves_name_position_and_created_at(
    reader: PReader, tmp_file: Path, tmp_path: Path, consume: Callable[..., Any]
) -> None:
    state = consume(reader.bytes(tmp_file, state=TEST_STATE_NAME), 1).state
    state.save()

    moved = tmp_path / "moved.bin"
    moved.write_bytes(tmp_file.read_bytes())

    resynced = state.resync(moved)

    assert resynced.name == state.name
    assert resynced.position == state.position
    assert resynced.timestamps.created_at == state.timestamps.created_at
    assert resynced.timestamps.updated_at > state.timestamps.updated_at


def test_resync_does_not_mutate_original(
    reader: PReader, tmp_file: Path, tmp_path: Path
) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    state.save()

    moved = tmp_path / "moved.bin"
    moved.write_bytes(tmp_file.read_bytes())

    state.resync(moved)

    assert state.file.path == tmp_file


@pytest.mark.parametrize(
    ("name", "message"),
    [("../../etc/passwd", "path escapes root"), (TEST_STATE_NAME, "io failed")],
    ids=["path_rejected", "commit_failed"],
)
def test_save_leaves_the_state_untouched_when_it_fails(
    config: Config,
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    name: str,
    message: str,
) -> None:
    state_dir = config.state_dir

    state_dir.mkdir()

    (state_dir / f"{TEST_STATE_NAME}.state.json").mkdir()

    state = make_reader().bytes(tmp_file, state=name).state
    before = state.timestamps.updated_at
    checksum = state.checksum()

    with pytest.raises(StateError, match=message):
        state.save()

    assert state.timestamps.updated_at == before
    assert state.checksum() == checksum


@pytest.mark.parametrize(
    "blocked", [False, True], ids=["when_it_succeeds", "when_it_fails"]
)
def test_save_leaves_no_temporary_file(
    config: Config, make_reader: Callable[..., PReader], tmp_file: Path, blocked: bool
) -> None:
    state_dir = config.state_dir

    state_dir.mkdir()

    if blocked:
        (state_dir / f"{TEST_STATE_NAME}.state.json").mkdir()

    state = make_reader().bytes(tmp_file, state=TEST_STATE_NAME).state

    if blocked:
        with pytest.raises(StateError, match="io failed"):
            state.save()
    else:
        state.save()

    assert list(state_dir.glob("*.tmp")) == []


def test_save_succeeds_after_the_file_is_deleted(
    reader: PReader, make_file: Callable[..., Path], read_state: Callable[..., Any]
) -> None:
    path = make_file(b"foo")
    state = reader.bytes(path, state=TEST_STATE_NAME).state

    path.unlink()

    assert state.save().exists()
    assert read_state(state)


def test_reload_raises_when_the_file_is_deleted(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo")
    state = reader.bytes(path, state=TEST_STATE_NAME).state

    path.unlink()
    state.save()

    with pytest.raises(StateError, match="NotFound"):
        reader.states[TEST_STATE_NAME]


def test_verify_suggests_resync_when_the_file_changed(
    make_reader: Callable[..., PReader],
    tmp_large_file: Path,
    append: Callable[[Path, bytes], None],
) -> None:
    state = _new_unverified_state(make_reader, tmp_large_file)

    append(tmp_large_file, b"more")

    with pytest.raises(StateError, match=r"call state\.resync\(file\)"):
        state.verify()


@pytest.mark.parametrize(
    ("before", "after"),
    [
        (b"foo\n" * 25, b"foo\n" * 2),
        (b"foo\n" * 2560, b"foo\n" * 250),
        (b"foo\n" * 2560, b"bar\n" * 1250),
        (b"foo\n" * 25, b"bar\n" * 27),
        (b"foo\n" * 25, b"bar\n" * 25),
        (b"foo\n" * 25, b""),
        (b"a" * 4096, b"a" * 4095),
    ],
    ids=[
        "truncated_inside_the_window",
        "truncated_inside_a_wide_window",
        "truncated_with_a_changed_prefix",
        "grown_with_a_changed_prefix",
        "replaced_at_the_same_size",
        "emptied",
        "truncated_at_the_window",
    ],
)
def test_resync_raises_when_the_prefix_is_gone(
    reader: PReader,
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    before: bytes,
    after: bytes,
) -> None:
    path = make_file(before, name="tracked.bin")
    state = consume(reader.bytes(path, state=TEST_STATE_NAME), 5).state

    make_file(after, name="tracked.bin")

    with pytest.raises(StateError, match="file content differs from the tracked file"):
        state.resync(path)


@pytest.mark.parametrize(
    ("before", "after"),
    [
        (b"foo\n" * 25, b"foo\n" * 25 + b"more"),
        (b"foo\n" * 25, b"foo\n" * 25),
        (b"foo\n" * 2560, b"foo\n" * 2560 + b"more"),
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
    path = make_file(before, name="tracked.bin")
    state = consume(reader.bytes(path, state=TEST_STATE_NAME), 5).state

    make_file(after, name="tracked.bin")

    resynced = state.resync(path)

    resynced.verify()

    assert resynced.position == 5
    assert resynced.file.size == len(after)
    assert resynced.position <= resynced.file.size


def test_resync_clamps_the_position_to_the_new_size(
    reader: PReader,
    tmp_large_file: Path,
    consume: Callable[..., Any],
    truncate: Callable[[Path, int], None],
) -> None:
    state = consume(reader.bytes(tmp_large_file, state=TEST_STATE_NAME), 6000).state

    truncate(tmp_large_file, 4500)

    resynced = state.resync(tmp_large_file)

    resynced.verify()

    assert state.position == 6000
    assert resynced.position == 4500
    assert resynced.percent() == 100.0


def test_resync_blind_spot_beyond_4096_bytes(
    reader: PReader,
    tmp_large_file: Path,
    consume: Callable[..., Any],
    overwrite: Callable[[Path, int, bytes], None],
) -> None:
    state = consume(reader.bytes(tmp_large_file, state=TEST_STATE_NAME), 6000).state

    overwrite(tmp_large_file, 4500, b"\xff")

    resynced = state.resync(tmp_large_file)

    resynced.verify()

    assert resynced.position == 6000


def test_resync_accepts_anything_for_an_empty_file(
    reader: PReader, empty_file: Path, make_file: Callable[..., Path]
) -> None:
    state = reader.bytes(empty_file, state=TEST_STATE_NAME).state
    replacement = make_file(b"foo", name="empty.bin")
    resynced = state.resync(replacement)

    assert resynced.position == 0
    assert resynced.file.size == 3


@pytest.mark.parametrize(
    ("after", "percent"),
    [(b"bar\n" * 25, 50.0), (b"foo\n" * 2, 625.0), (b"", 5000.0)],
    ids=["replaced", "shrunk", "emptied"],
)
def test_resync_trusts_the_path_without_verification(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
    after: bytes,
    percent: float,
) -> None:
    reader = make_reader(verify_state=False)
    path = make_file(b"foo\n" * 25, name="tracked.bin")
    state = consume(reader.bytes(path, state=TEST_STATE_NAME), 50).state

    make_file(after, name="tracked.bin")

    resynced = state.resync(path)

    assert resynced.position == 50
    assert resynced.file.size == len(after)
    assert resynced.percent() == percent


@pytest.mark.parametrize(
    ("before", "after", "read", "position", "percent"),
    [(4096, 4096, 100, 100, 100 * 100 / 4096), (4097, 4096, 4097, 4096, 100.0)],
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
    path = make_file(b"a" * before, name="tracked.bin")
    state = consume(reader.bytes(path, state=TEST_STATE_NAME), read).state

    make_file(b"a" * after, name="tracked.bin")

    resynced = state.resync(path)

    assert resynced.position == position
    assert resynced.percent() == percent
    assert resynced.position <= resynced.file.size


def test_resync_keeps_an_unsafe_name_for_the_save(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\n" * 25, name="tracked.bin")
    state = reader.bytes(path, state="../../etc/passwd").state
    resynced = state.resync(path)

    assert resynced.name == "../../etc/passwd"

    with pytest.raises(StateError, match="path escapes root"):
        resynced.save()


@pytest.mark.usefixtures("requires_pre_epoch_mtime")
def test_resync_raises_when_mtime_precedes_the_epoch(
    reader: PReader, tmp_large_file: Path
) -> None:
    state = reader.bytes(tmp_large_file, state=TEST_STATE_NAME).state

    os.utime(tmp_large_file, (-86400, -86400))

    with pytest.raises(StateError, match="second time provided"):
        state.resync(tmp_large_file)


@pytest.mark.usefixtures("requires_symlinks")
@pytest.mark.parametrize(
    "linked", [True, False], ids=["a_symlink", "an_unnormalized_path"]
)
def test_resync_canonicalizes_the_path(
    reader: PReader, tmp_path: Path, make_file: Callable[..., Path], linked: bool
) -> None:
    target = make_file(b"foo\n" * 25, name="real.bin")
    state = reader.bytes(target, state=TEST_STATE_NAME).state
    detour = tmp_path / "." / "real.bin"

    if linked:
        detour = tmp_path / "alias.bin"
        detour.symlink_to(target)

    assert state.resync(detour).file.path == target


def test_resync_keeps_a_position_that_equals_the_size(
    reader: PReader, make_file: Callable[..., Path], consume: Callable[..., Any]
) -> None:
    path = make_file(b"foo\n" * 25, name="tracked.bin")
    state = consume(reader.bytes(path, state=TEST_STATE_NAME)).state
    resynced = state.resync(path)

    assert state.position == 100
    assert resynced.position == 100
    assert resynced.file.size == 100


def test_resync_raises_when_the_file_is_unreadable(
    reader: PReader, tmp_file: Path, revoke_permissions: Callable[[Path], Path]
) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state

    revoke_permissions(tmp_file)

    with pytest.raises(StateError, match="Permission denied"):
        state.resync(tmp_file)


def test_resync_keeps_the_state_dir_for_the_save(
    reader: PReader, config: Config, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\n" * 25, name="tracked.bin")
    state = reader.bytes(path, state=TEST_STATE_NAME).state
    written = state.resync(path).save()

    assert written.parent == config.state_dir
    assert written.is_file()
    assert reader.states[TEST_STATE_NAME].name == TEST_STATE_NAME


@pytest.mark.parametrize(
    "name", ["missing.bin", "elsewhere"], ids=["missing", "a_directory"]
)
def test_resync_still_checks_the_path_without_verification(
    make_reader: Callable[..., PReader], tmp_file: Path, tmp_path: Path, name: str
) -> None:
    reader = make_reader(verify_state=False)
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    target = tmp_path / name

    if name == "elsewhere":
        target.mkdir()

    with pytest.raises(StateError):
        state.resync(target)


def test_resync_accepts_every_path_shape(
    reader: PReader, tmp_file: Path, consume: Callable[..., Any]
) -> None:
    state = consume(reader.bytes(tmp_file, state=TEST_STATE_NAME), 1).state

    for shape in (str(tmp_file), tmp_file):
        assert state.resync(shape).file.path == tmp_file


def test_resync_moves_the_identity_reference_forward(
    reader: PReader, make_file: Callable[..., Path], consume: Callable[..., Any]
) -> None:
    original = make_file(b"foo\n" * 25, name="tracked.bin")
    grown = make_file(b"foo\n" * 30, name="grown.bin")

    state = consume(reader.bytes(original, state=TEST_STATE_NAME), 5).state
    once = state.resync(grown)

    assert once.file.size == 120

    with pytest.raises(StateError, match="file content differs from the tracked file"):
        once.resync(original)

    with pytest.raises(StateError, match=r"grown\.bin.*tracked\.bin"):
        once.resync(original)


def test_resync_is_stable_when_repeated(
    reader: PReader, make_file: Callable[..., Path], consume: Callable[..., Any]
) -> None:
    path = make_file(b"foo\n" * 25, name="tracked.bin")
    state = consume(reader.bytes(path, state=TEST_STATE_NAME), 5).state
    once = state.resync(path)
    twice = once.resync(path)

    twice.verify()

    assert twice.position == 5
    assert twice.file == once.file


@pytest.mark.usefixtures("requires_non_utf8_names")
def test_resync_raises_when_path_is_not_utf8(
    reader: PReader, tmp_file: Path, make_file: Callable[..., Path]
) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    odd = make_file(tmp_file.read_bytes(), name=os.fsdecode(b"data-\xff.bin"))

    with pytest.raises(StateError, match="invalid UTF-8"):
        state.resync(odd)


def test_resync_reseals_a_tampered_state(
    make_reader: Callable[..., PReader], make_file: Callable[..., Path]
) -> None:
    reader = make_reader(verify_state=False)
    path = make_file(b"a" * 20)
    state_path = reader.bytes(path, state=TEST_STATE_NAME).state.save()

    payload = json.loads(state_path.read_text())
    payload["position"] = 999

    state_path.write_text(json.dumps(payload))

    tampered = reader.states[TEST_STATE_NAME]

    with pytest.raises(StateError, match="checksum mismatch"):
        tampered.verify()

    resynced = tampered.resync(path)

    resynced.verify()

    assert resynced.position == 999


def test_resynced_state_is_still_rejected_for_another_file(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    first = make_file(b"a" * 10, name="a.bin")
    second = make_file(b"b" * 10, name="b.bin")
    resynced = reader.bytes(first, state=TEST_STATE_NAME).state.resync(first)

    with pytest.raises(
        StateError, match=r"file path mismatch .*call state\.resync\(file\)"
    ):
        reader.bytes(second, state=resynced)


@pytest.mark.parametrize("linked", [False, True], ids=["directly", "through_a_symlink"])
def test_resync_raises_when_the_path_is_a_directory(
    reader: PReader, tmp_file: Path, tmp_path: Path, linked: bool
) -> None:
    state = reader.bytes(tmp_file).state

    directory = tmp_path / "elsewhere"
    directory.mkdir()

    target = directory

    if linked:
        target = tmp_path / "alias"
        target.symlink_to(directory, target_is_directory=True)

    with pytest.raises(StateError, match="not a file"):
        state.resync(target)


@pytest.mark.parametrize("looped", [False, True], ids=["missing", "a_symlink_loop"])
def test_resync_raises_when_the_path_cannot_be_resolved(
    reader: PReader, tmp_file: Path, tmp_path: Path, looped: bool
) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    target = tmp_path / "does-not-exist.bin"

    if looped:
        target = tmp_path / "a-link"
        second = tmp_path / "b-link"

        target.symlink_to(second)
        second.symlink_to(target)

    with pytest.raises(StateError, match="io failed"):
        state.resync(target)


def test_resync_allows_resuming_moved_file(
    reader: PReader, tmp_file: Path, tmp_path: Path, consume: Callable[..., Any]
) -> None:
    content = tmp_file.read_bytes()
    state = consume(reader.bytes(tmp_file, state=TEST_STATE_NAME), 1).state
    state.save()

    moved = tmp_file.rename(tmp_path / "moved.bin")

    with pytest.raises(StateError, match="resync"):
        reader.bytes(moved, state=state)

    resynced = state.resync(moved)
    resumed = reader.bytes(moved, state=resynced)

    assert resumed.state.file.path == moved
    assert b"".join(resumed) == content[1:]


def test_resync_allows_resuming_grown_file(
    reader: PReader,
    tmp_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    state = consume(reader.bytes(tmp_file, state=TEST_STATE_NAME)).state
    state.save()

    append(tmp_file, b"more")

    with pytest.raises(StateError, match="size mismatch"):
        reader.bytes(tmp_file, state=state)

    resynced = state.resync(tmp_file)
    resumed = reader.bytes(tmp_file, state=resynced)

    assert resynced.file.size == len(tmp_file.read_bytes())
    assert b"".join(resumed) == b"more"


def test_state_compares_by_value(
    reader: PReader, tmp_file: Path, tmp_large_file: Path
) -> None:
    iterator = reader.bytes(tmp_file)

    assert iterator.state is not iterator.state
    assert iterator.state == iterator.state
    assert iterator.state != reader.bytes(tmp_large_file).state


def test_state_repr(
    reader: PReader,
    tmp_file: Path,
    reindent: Callable[[str, int], str],
    expected_repr: Callable[..., str],
) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state

    assert repr(state) == expected_repr(
        "State",
        name=f"'{state.name}'",
        file=reindent(repr(state.file), 2),
        position=state.position,
        timestamps=reindent(repr(state.timestamps), 2),
    )


def test_name_drops_the_suffix(reader: PReader, tmp_file: Path) -> None:
    state = reader.bytes(tmp_file, state=f"{TEST_STATE_NAME}.state.json").state

    assert state.name == TEST_STATE_NAME
    assert state.path().name == f"{TEST_STATE_NAME}.state.json"


def test_save_accepts_a_unicode_name(
    reader: PReader, tmp_file: Path, read_state: Callable[..., Any]
) -> None:
    state = reader.bytes(tmp_file, state="job-café").state

    assert state.save().name == "job-café.state.json"
    assert read_state(state)["name"] == "job-café"


def test_state_getter_returns_an_independent_snapshot(
    reader: PReader, tmp_large_file: Path
) -> None:
    iterator = reader.bytes(tmp_large_file)
    snapshot = iterator.state

    next(iterator)

    assert snapshot is not iterator.state
    assert snapshot.position == 0
    assert iterator.state.position == 1


def test_percent_exceeds_hundred_past_the_file_size(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)
    path = make_file(b"foo")
    state_path = consume(reader.bytes(path, state=TEST_STATE_NAME)).state.save()

    payload = json.loads(state_path.read_text())
    payload["position"] = 999

    state_path.write_text(json.dumps(payload))

    assert reader.states[TEST_STATE_NAME].percent() == 33300.0


def test_percent_treats_a_zero_size_as_one_byte(
    make_reader: Callable[..., PReader],
    make_file: Callable[..., Path],
    consume: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)
    path = make_file(b"foo")
    state_path = consume(reader.bytes(path, state=TEST_STATE_NAME)).state.save()

    payload = json.loads(state_path.read_text())
    payload["file"]["size"] = 0
    payload["position"] = 5

    state_path.write_text(json.dumps(payload))

    assert reader.states[TEST_STATE_NAME].percent() == 500.0


def test_save_raises_when_the_name_is_too_long(reader: PReader, tmp_file: Path) -> None:
    state = reader.bytes(tmp_file, state="x" * 300).state

    with pytest.raises(StateError, match="io failed"):
        state.save()


def test_save_does_not_disturb_the_iterator(
    reader: PReader, tmp_large_file: Path, consume: Callable[..., Any]
) -> None:
    iterator = consume(reader.bytes(tmp_large_file, state=TEST_STATE_NAME), 5)

    iterator.state.save()

    assert reader.states[TEST_STATE_NAME].position == 5
    assert len(b"".join(iterator)) == len(tmp_large_file.read_bytes()) - 5


def test_save_syncs_the_state_object_with_the_file(
    reader: PReader,
    tmp_large_file: Path,
    consume: Callable[..., Any],
    read_state: Callable[..., Any],
) -> None:
    iterator = consume(reader.bytes(tmp_large_file, state=TEST_STATE_NAME), 12)
    state = iterator.state

    state.save()

    payload = read_state(state)

    assert payload["position"] == 12
    assert payload["_checksum"] == state.checksum()

    reloaded = reader.states[TEST_STATE_NAME]

    assert reloaded.name == state.name
    assert reloaded.position == 12
    assert reloaded.file == state.file
    assert reloaded.checksum() == state.checksum()
    assert reloaded.timestamps.created_at == state.timestamps.created_at.replace(
        microsecond=0
    )
    assert reloaded.timestamps.updated_at == state.timestamps.updated_at.replace(
        microsecond=0
    )


def test_saved_payload_has_the_expected_keys(
    reader: PReader, tmp_file: Path, read_state: Callable[..., Any]
) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    state.save()

    payload = read_state(state)

    assert sorted(payload) == ["_checksum", "file", "name", "position", "timestamps"]
    assert sorted(payload["file"]) == ["fingerprint", "mtime", "path", "size"]
    assert sorted(payload["timestamps"]) == ["created_at", "updated_at"]


def test_save_refreshes_the_checksum(
    reader: PReader,
    tmp_large_file: Path,
    consume: Callable[..., Any],
    read_state: Callable[..., Any],
) -> None:
    iterator = reader.bytes(tmp_large_file, state=TEST_STATE_NAME)
    before = iterator.state.checksum()

    consume(iterator, 5)

    state = iterator.state

    state.save()

    assert reader.states[TEST_STATE_NAME].checksum() != before
    assert read_state(state)["_checksum"] != before


def test_save_is_relative_without_a_state_dir(
    tmp_file: Path,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    read_state: Callable[..., Any],
) -> None:
    monkeypatch.chdir(tmp_path)
    reader = PReader(config=Config(state_dir=""))

    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    path = state.save()

    assert str(path) == f"{TEST_STATE_NAME}.state.json"
    assert (tmp_path / path).exists()
    assert read_state(state)


def test_verify_reports_the_checksum_first(
    make_reader: Callable[..., PReader],
    tmp_large_file: Path,
    truncate: Callable[[Path, int], None],
) -> None:
    state = _new_unverified_state(make_reader, tmp_large_file)
    original_mtime = tmp_large_file.stat().st_mtime
    payload = json.loads(state.path().read_text())

    payload.update(position=999)
    state.path().write_text(json.dumps(payload))

    truncate(tmp_large_file, 4500)

    os.utime(tmp_large_file, (original_mtime, original_mtime))

    with pytest.raises(StateError, match="checksum mismatch"):
        _get_unverified_state(make_reader, state.name).verify()
