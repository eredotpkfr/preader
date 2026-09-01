import json
import os
import re

import pytest

from preader import Config, PReader, StateError

from constants import (
    TEST_STATE_NAME,
    TEST_UNSAFE_STATE_NAMES,
    TEST_UNSAFE_STATE_NAME_IDS,
    TEST_WINDOWS_UNSAFE_STATE_NAMES,
    TEST_WINDOWS_UNSAFE_STATE_NAME_IDS,
)


def test_state_fields(config, reader, tmp_file):
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state

    assert state.name == TEST_STATE_NAME
    assert state.position == 0
    assert state.file.path == tmp_file
    assert state.path == config.state_dir / f"{state.name}.state.json"
    assert state.timestamps.created_at == state.timestamps.updated_at


@pytest.mark.parametrize(
    ("name", "message"), TEST_UNSAFE_STATE_NAMES.items(), ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_path_raises_when_name_is_unsafe(reader, tmp_file, name, message):
    state = reader.bytes(tmp_file, state=name).state

    with pytest.raises(StateError, match=message):
        state.path


@pytest.mark.skipif(os.name != "nt", reason="Windows path syntax is only unsafe on Windows")
@pytest.mark.parametrize(
    "name", TEST_WINDOWS_UNSAFE_STATE_NAMES, ids=TEST_WINDOWS_UNSAFE_STATE_NAME_IDS
)
def test_path_raises_when_a_windows_name_is_unsafe(reader, tmp_file, name):
    state = reader.bytes(tmp_file, state=name).state

    with pytest.raises(StateError, match="path escapes root"):
        state.path


def test_checksum_is_stable(reader, tmp_file):
    state = reader.bytes(tmp_file).state
    checksum = state.checksum()

    assert re.fullmatch(r"[0-9a-f]{64}", checksum)
    assert state.checksum() == checksum


@pytest.mark.parametrize("content", [b"", b"foo"], ids=["empty_file", "unread_file"])
def test_percent_before_reading(reader, make_file, content):
    assert reader.bytes(make_file(content)).percent() == 0.0


def test_bytes_read_match_file_content(reader, tmp_file):
    content = tmp_file.read_bytes()
    read_bytes = list(reader.bytes(tmp_file))

    assert all(len(chunk) == 1 for chunk in read_bytes)
    assert b"".join(read_bytes) == content


def test_save_returns_created_path(reader, tmp_file):
    state = reader.bytes(tmp_file).state
    assert not state.path.exists()

    assert state.save() == state.path
    assert state.path.exists()


@pytest.mark.repeat(10)
def test_save_updates_timestamp_but_not_created_at(reader, tmp_file):
    state = reader.bytes(tmp_file).state
    created_at = state.timestamps.created_at

    state.save()
    state.save()

    assert state.timestamps.created_at == created_at
    assert state.timestamps.updated_at > created_at


def test_save_persists_across_new_reader(make_reader, reader, tmp_file, consume):
    iterator = consume(reader.bytes(tmp_file, state=TEST_STATE_NAME))

    iterator.state.save()

    new_reader = make_reader()
    loaded = new_reader.states[TEST_STATE_NAME]

    assert loaded.position == len(tmp_file.read_bytes())


def test_save_raises_when_state_dir_blocked(config, make_reader, tmp_file):
    config.state_dir.write_bytes(b"foo")

    state = make_reader().bytes(tmp_file).state

    with pytest.raises(StateError, match="AlreadyExists"):
        state.save()


def test_save_raises_when_state_path_is_a_directory(reader, tmp_file):
    state = reader.bytes(tmp_file).state
    state.path.mkdir(parents=True)

    with pytest.raises(StateError, match="io failed"):
        state.save()


def _get_unverified_state(make_reader, name):
    return make_reader(verify_state=False).states[name]


def _new_unverified_state(make_reader, data_file):
    reader = make_reader(verify_state=False)

    reader.bytes(data_file, state=TEST_STATE_NAME).state.save()

    return _get_unverified_state(make_reader, TEST_STATE_NAME)


def test_verify_passes_when_untouched(make_reader, tmp_large_file):
    assert _new_unverified_state(make_reader, tmp_large_file).verify() is None


def test_verify_raises_when_checksum_mismatch(make_reader, tmp_large_file):
    state = _new_unverified_state(make_reader, tmp_large_file)
    payload = json.loads(state.path.read_text())

    payload.update(position=999)
    state.path.write_text(json.dumps(payload))

    tampered = _get_unverified_state(make_reader, state.name)

    with pytest.raises(StateError, match="checksum mismatch"):
        tampered.verify()


def test_verify_raises_when_size_mismatch(make_reader, tmp_large_file):
    state = _new_unverified_state(make_reader, tmp_large_file)
    original_mtime = os.stat(tmp_large_file).st_mtime

    with open(tmp_large_file, "r+b") as f:
        f.truncate(4500)

    os.utime(tmp_large_file, (original_mtime, original_mtime))

    with pytest.raises(StateError, match="file size mismatch"):
        state.verify()


@pytest.mark.usefixtures("requires_pre_epoch_mtime")
def test_verify_raises_when_mtime_precedes_the_epoch(make_reader, tmp_large_file):
    state = _new_unverified_state(make_reader, tmp_large_file)

    os.utime(tmp_large_file, (-86400, -86400))

    with pytest.raises(StateError, match="io failed"):
        state.verify()


def test_verify_raises_when_mtime_mismatch(make_reader, tmp_large_file):
    state = _new_unverified_state(make_reader, tmp_large_file)
    original_mtime = os.stat(tmp_large_file).st_mtime

    os.utime(tmp_large_file, (original_mtime + 3600, original_mtime + 3600))

    with pytest.raises(StateError, match="file mtime mismatch"):
        state.verify()


def test_verify_raises_when_fingerprint_mismatch(make_reader, tmp_large_file):
    state = _new_unverified_state(make_reader, tmp_large_file)
    original_mtime = os.stat(tmp_large_file).st_mtime

    with open(tmp_large_file, "r+b") as f:
        f.seek(10)
        f.write(b"\xff")

    os.utime(tmp_large_file, (original_mtime, original_mtime))

    with pytest.raises(StateError, match="file fingerprint mismatch"):
        state.verify()


def test_verify_blind_spot_beyond_4096_bytes(make_reader, tmp_large_file):
    state = _new_unverified_state(make_reader, tmp_large_file)
    original_mtime = os.stat(tmp_large_file).st_mtime

    with open(tmp_large_file, "r+b") as f:
        f.seek(4500)
        f.write(b"\xff")

    os.utime(tmp_large_file, (original_mtime, original_mtime))

    assert state.verify() is None


def test_verify_raises_when_file_deleted(make_reader, tmp_large_file):
    state = _new_unverified_state(make_reader, tmp_large_file)

    os.remove(tmp_large_file)

    with pytest.raises(StateError, match="NotFound") as exc_info:
        state.verify()

    assert "mismatch" not in str(exc_info.value)


def test_resync_updates_file_metadata(reader, tmp_file, tmp_path, fingerprint):
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    state.save()

    moved = tmp_path / "moved.bin"
    moved.write_bytes(tmp_file.read_bytes())

    resynced = state.resync(moved)

    assert resynced.file.path == moved
    assert resynced.file.size == len(moved.read_bytes())
    assert resynced.file.mtime.timestamp() == int(moved.stat().st_mtime)
    assert resynced.file.fingerprint == fingerprint(moved)


def test_resync_preserves_name_position_and_created_at(reader, tmp_file, tmp_path, consume):
    state = consume(reader.bytes(tmp_file, state=TEST_STATE_NAME), 1).state
    state.save()

    moved = tmp_path / "moved.bin"
    moved.write_bytes(tmp_file.read_bytes())

    resynced = state.resync(moved)

    assert resynced.name == state.name
    assert resynced.position == state.position
    assert resynced.timestamps.created_at == state.timestamps.created_at


def test_resync_does_not_mutate_original(reader, tmp_file, tmp_path):
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
    config, make_reader, tmp_file, name, message
):
    state_dir = config.state_dir

    state_dir.mkdir()

    (state_dir / f"{TEST_STATE_NAME}.state.json").mkdir()

    state = make_reader().bytes(tmp_file, state=name).state
    before = state.timestamps.updated_at

    with pytest.raises(StateError, match=message):
        state.save()

    assert state.timestamps.updated_at == before


def test_save_removes_the_temporary_file_when_it_fails(config, make_reader, tmp_file):
    state_dir = config.state_dir

    state_dir.mkdir()

    (state_dir / f"{TEST_STATE_NAME}.state.json").mkdir()

    state = make_reader().bytes(tmp_file, state=TEST_STATE_NAME).state

    with pytest.raises(StateError, match="io failed"):
        state.save()

    assert list(state_dir.glob("*.tmp")) == []


def test_save_succeeds_after_the_file_is_deleted(reader, make_file):
    path = make_file(b"foo")
    state = reader.bytes(path, state=TEST_STATE_NAME).state

    path.unlink()

    assert state.save().exists()


def test_reload_raises_when_the_file_is_deleted(reader, make_file):
    path = make_file(b"foo")
    state = reader.bytes(path, state=TEST_STATE_NAME).state

    path.unlink()
    state.save()

    with pytest.raises(StateError, match="NotFound"):
        reader.states[TEST_STATE_NAME]


def test_verify_suggests_resync_when_the_file_changed(make_reader, tmp_large_file, append):
    state = _new_unverified_state(make_reader, tmp_large_file)

    append(tmp_large_file, b"more")

    with pytest.raises(StateError, match=r"call state\.resync\(file\)"):
        state.verify()


def test_resync_raises_when_the_path_is_a_directory(reader, tmp_file, tmp_path):
    state = reader.bytes(tmp_file).state

    directory = tmp_path / "elsewhere"
    directory.mkdir()

    with pytest.raises(StateError, match="not a file"):
        state.resync(directory)


def test_resync_raises_when_file_missing(reader, tmp_file, tmp_path):
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    state.save()

    with pytest.raises(StateError, match="NotFound"):
        state.resync(tmp_path / "does-not-exist.bin")


def test_resync_allows_resuming_moved_file(reader, tmp_file, tmp_path, consume):
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


def test_resync_allows_resuming_grown_file(reader, tmp_file, consume, append):
    state = consume(reader.bytes(tmp_file, state=TEST_STATE_NAME)).state
    state.save()

    append(tmp_file, b"more")

    with pytest.raises(StateError, match="size mismatch"):
        reader.bytes(tmp_file, state=state)

    resynced = state.resync(tmp_file)
    resumed = reader.bytes(tmp_file, state=resynced)

    assert resynced.file.size == len(tmp_file.read_bytes())
    assert b"".join(resumed) == b"more"


def test_state_repr(reader, tmp_file, reindent, expected_repr):
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state

    assert repr(state) == expected_repr(
        "State",
        name=f"'{state.name}'",
        file=reindent(repr(state.file), 2),
        position=state.position,
        timestamps=reindent(repr(state.timestamps), 2),
    )


def test_name_drops_the_suffix(reader, tmp_file):
    state = reader.bytes(tmp_file, state=f"{TEST_STATE_NAME}.state.json").state

    assert state.name == TEST_STATE_NAME
    assert state.path.name == f"{TEST_STATE_NAME}.state.json"


def test_save_accepts_a_unicode_name(reader, tmp_file):
    state = reader.bytes(tmp_file, state="job-café").state

    assert state.save().name == "job-café.state.json"


def test_state_getter_returns_an_independent_snapshot(reader, tmp_large_file):
    iterator = reader.bytes(tmp_large_file)
    snapshot = iterator.state

    next(iterator)

    assert snapshot is not iterator.state
    assert snapshot.position == 0
    assert iterator.state.position == 1


def test_percent_exceeds_hundred_past_the_file_size(make_reader, make_file, consume):
    reader = make_reader(verify_state=False)
    path = make_file(b"foo")
    state_path = consume(reader.bytes(path, state=TEST_STATE_NAME)).state.save()

    payload = json.loads(state_path.read_text())
    payload["position"] = 999

    state_path.write_text(json.dumps(payload))

    assert reader.states[TEST_STATE_NAME].percent() == 33300.0


def test_percent_treats_a_zero_size_as_one_byte(make_reader, make_file, consume):
    reader = make_reader(verify_state=False)
    path = make_file(b"foo")
    state_path = consume(reader.bytes(path, state=TEST_STATE_NAME)).state.save()

    payload = json.loads(state_path.read_text())
    payload["file"]["size"] = 0
    payload["position"] = 5

    state_path.write_text(json.dumps(payload))

    assert reader.states[TEST_STATE_NAME].percent() == 500.0


def test_save_raises_when_the_name_is_too_long(reader, tmp_file):
    state = reader.bytes(tmp_file, state="x" * 300).state

    with pytest.raises(StateError, match="io failed"):
        state.save()


def test_save_does_not_disturb_the_iterator(reader, tmp_large_file, consume):
    iterator = consume(reader.bytes(tmp_large_file, state=TEST_STATE_NAME), 5)

    iterator.state.save()

    assert reader.states[TEST_STATE_NAME].position == 5
    assert len(b"".join(iterator)) == len(tmp_large_file.read_bytes()) - 5


def test_saved_payload_has_the_expected_keys(reader, tmp_file):
    payload = json.loads(reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save().read_text())

    assert sorted(payload) == ["_checksum", "file", "name", "position", "timestamps"]
    assert sorted(payload["file"]) == ["fingerprint", "mtime", "path", "size"]
    assert sorted(payload["timestamps"]) == ["created_at", "updated_at"]


def test_save_refreshes_the_checksum(reader, tmp_large_file, consume):
    iterator = reader.bytes(tmp_large_file, state=TEST_STATE_NAME)
    before = iterator.state.checksum()

    consume(iterator, 5)
    iterator.state.save()

    assert reader.states[TEST_STATE_NAME].checksum() != before


def test_save_is_relative_without_a_state_dir(tmp_file, tmp_path, monkeypatch):
    monkeypatch.chdir(tmp_path)
    reader = PReader(config=Config(state_dir=""))

    path = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert str(path) == f"{TEST_STATE_NAME}.state.json"
    assert (tmp_path / path).exists()


def test_verify_reports_the_checksum_first(make_reader, tmp_large_file):
    state = _new_unverified_state(make_reader, tmp_large_file)
    original_mtime = os.stat(tmp_large_file).st_mtime
    payload = json.loads(state.path.read_text())

    payload.update(position=999)
    state.path.write_text(json.dumps(payload))

    with open(tmp_large_file, "r+b") as f:
        f.truncate(4500)

    os.utime(tmp_large_file, (original_mtime, original_mtime))

    with pytest.raises(StateError, match="checksum mismatch"):
        _get_unverified_state(make_reader, state.name).verify()
