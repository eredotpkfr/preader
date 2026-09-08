import json
import os
import warnings

from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

from constants import (
    TEST_ALPHABET,
    TEST_STATE_NAME,
    TEST_UNSAFE_STATE_NAME_IDS,
    TEST_UNSAFE_STATE_NAMES,
    TEST_WINDOWS_UNSAFE_STATE_NAME_IDS,
    TEST_WINDOWS_UNSAFE_STATE_NAMES,
)
from preader import Config, IteratorOptions, PReader, SaveWarning, State, StateError


@pytest.fixture
def data_file(make_file: Callable[..., Path]) -> Path:
    return make_file(TEST_ALPHABET)


@pytest.mark.parametrize(
    ("options", "expected"),
    [
        (IteratorOptions(start=6), TEST_ALPHABET[6:]),
        (IteratorOptions(end=10), TEST_ALPHABET[:10]),
        (IteratorOptions(skip=4), TEST_ALPHABET[12:]),
        (IteratorOptions(start=6, limit=2), TEST_ALPHABET[6:12]),
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
    assert b"".join(reader.chunks(data_file, options=options, chunk_size=3)) == expected


@pytest.mark.parametrize("buffer_capacity", [0, 2], ids=["zero", "tiny"])
def test_buffer_capacity_smaller_than_chunk_size(
    data_file: Path, make_reader: Callable[..., PReader], buffer_capacity: int
) -> None:
    reader = make_reader(buffer_capacity=buffer_capacity)

    assert b"".join(reader.chunks(data_file, chunk_size=10)) == TEST_ALPHABET


def test_zero_chunk_size_yields_nothing(reader: PReader, tmp_file: Path) -> None:
    assert list(reader.chunks(tmp_file, chunk_size=0)) == []


def test_drop_partial_discards_chunk_cut_short_by_end(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(end=12)
    iterator = reader.chunks(
        data_file, options=options, chunk_size=5, drop_partial=True
    )

    assert list(iterator) == [TEST_ALPHABET[0:5], TEST_ALPHABET[5:10]]
    assert iterator.state.position == 10


def test_keeps_partial_chunk_cut_by_end(reader: PReader, data_file: Path) -> None:
    options = IteratorOptions(end=12)

    chunks = list(reader.chunks(data_file, options=options, chunk_size=5))

    assert chunks == [TEST_ALPHABET[0:5], TEST_ALPHABET[5:10], TEST_ALPHABET[10:12]]


def test_resume_with_different_chunk_size(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    state = consume(
        reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=4), 1
    ).state
    state.save()

    resumed = list(reader.chunks(data_file, state=state, chunk_size=3))

    remaining = TEST_ALPHABET[4:]
    expected = [remaining[i : i + 3] for i in range(0, len(remaining), 3)]

    assert resumed == expected


def test_resume_ignores_options_when_already_past_start(
    reader: PReader, data_file: Path, consume: Callable[..., Any]
) -> None:
    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3)
    options = IteratorOptions(start=3)

    state = consume(iterator, 4).state
    state.save()

    resumed = reader.chunks(data_file, state=state, options=options, chunk_size=3)

    assert b"".join(resumed) == TEST_ALPHABET[12:]


def test_end_below_the_position_does_not_rewind_the_state(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(
        auto_save_state=True, auto_save_state_bytes=64, auto_load_state=True
    )

    list(reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3))

    saved = reader.states[TEST_STATE_NAME].position

    assert saved == len(data_file.read_bytes())

    list(
        reader.chunks(
            data_file,
            state=TEST_STATE_NAME,
            options=IteratorOptions(end=5),
            chunk_size=3,
        )
    )

    assert reader.states[TEST_STATE_NAME].position == saved


def test_resume_on_fully_consumed_file_yields_nothing(
    reader: PReader, tmp_file: Path, consume: Callable[..., Any]
) -> None:
    iterator = reader.chunks(tmp_file, state=TEST_STATE_NAME, chunk_size=3)

    state = consume(iterator).state
    state.save()

    resumed = reader.chunks(tmp_file, state=state, chunk_size=3)

    assert list(resumed) == []
    assert resumed.percent() == 100.0


def test_threshold_autosave_triggers_mid_iteration(
    data_file: Path, make_reader: Callable[..., PReader], consume: Callable[..., Any]
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=9)

    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3)
    consume(iterator, 3)

    assert reader.states[TEST_STATE_NAME].position == 9


def test_drop_saves_partial_progress(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True)

    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3)

    next(iterator)

    del iterator

    assert reader.states[TEST_STATE_NAME].position == 3


def test_drop_does_not_save_when_auto_save_disabled(
    reader: PReader, tmp_file: Path
) -> None:
    iterator = reader.chunks(tmp_file, state=TEST_STATE_NAME, chunk_size=3)

    next(iterator)

    del iterator

    assert TEST_STATE_NAME not in reader.states


def test_zero_threshold_saves_only_at_finalize(
    data_file: Path, make_reader: Callable[..., PReader], consume: Callable[..., Any]
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=0)
    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3)

    consume(iterator, 3)
    position = iterator.state.position

    assert TEST_STATE_NAME not in reader.states

    del iterator

    assert reader.states[TEST_STATE_NAME].position == position


def test_extra_next_after_exhaustion_does_not_resave(
    data_file: Path, make_reader: Callable[..., PReader], consume: Callable[..., Any]
) -> None:
    reader = make_reader(auto_save_state=True)
    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3)

    consume(iterator)

    mtime_before = reader.states[TEST_STATE_NAME].path().stat().st_mtime

    with pytest.raises(StopIteration):
        next(iterator)

    assert reader.states[TEST_STATE_NAME].path().stat().st_mtime == mtime_before


def test_a_failing_autosave_warns_and_keeps_every_item(
    config: Config,
    make_reader: Callable[..., PReader],
    data_file: Path,
) -> None:
    config.state_dir.write_bytes(b"foo")

    reader = make_reader(auto_save_state=True, auto_save_state_bytes=5)

    with pytest.warns(SaveWarning):
        consumed = b"".join(
            reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3)
        )

    assert consumed == TEST_ALPHABET


def test_a_failing_final_save_keeps_every_item(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True)

    assert b"".join(reader.chunks(data_file, state="../../etc/passwd")) == TEST_ALPHABET


def test_a_failing_autosave_keeps_the_items_read_before_a_truncation(
    data_file: Path,
    make_reader: Callable[..., PReader],
    truncate: Callable[[Path, int], None],
) -> None:
    reader = make_reader(auto_save_state=True, buffer_capacity=1)
    iterator = reader.chunks(data_file, chunk_size=3, state="../../etc/passwd")

    assert next(iterator) == TEST_ALPHABET[:3]

    truncate(data_file, 1)

    assert b"".join(iterator) == b""


def test_a_failing_autosave_keeps_every_item_when_end_drops_a_chunk(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True)
    options = IteratorOptions(end=12)
    chunks = reader.chunks(
        data_file,
        chunk_size=8,
        drop_partial=True,
        state="../../etc/passwd",
        options=options,
    )

    assert b"".join(chunks) == TEST_ALPHABET[:8]


@pytest.mark.parametrize("threshold", [4, 1000], ids=["at_the_threshold", "at_the_end"])
def test_a_failing_autosave_keeps_every_item_on_a_dropped_short_chunk(
    make_file: Callable[..., Path],
    make_reader: Callable[..., PReader],
    threshold: int,
    truncate: Callable[[Path, int], None],
) -> None:
    reader = make_reader(
        auto_save_state=True, auto_save_state_bytes=threshold, buffer_capacity=1
    )
    path = make_file(b"foobarbaz")
    iterator = reader.chunks(
        path, chunk_size=3, drop_partial=True, state="../../etc/passwd"
    )

    assert next(iterator) == b"foo"

    truncate(path, 5)

    assert b"".join(iterator) == b""


def test_a_failing_autosave_can_be_made_fatal(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=1)

    with warnings.catch_warnings():
        warnings.simplefilter("error", SaveWarning)

        with pytest.raises(SaveWarning, match="path escapes root"):
            list(reader.chunks(data_file, chunk_size=3, state="../../etc/passwd"))


def test_chunk_iterator_repr(
    reader: PReader,
    tmp_file: Path,
    reindent: Callable[[str, int], str],
    expected_repr: Callable[..., str],
) -> None:
    iterator = reader.chunks(tmp_file, chunk_size=1024, drop_partial=True)

    assert repr(iterator) == expected_repr(
        "ChunkIterator",
        state=reindent(repr(iterator.state), 2),
        chunk_size=iterator.chunk_size,
        drop_partial=str(iterator.drop_partial).lower(),
    )


def test_auto_load_state_resumes_previous_position(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_load_state=True)
    iterator = reader.chunks(data_file, chunk_size=5)

    next(iterator)

    iterator.state.save()

    assert reader.chunks(data_file, chunk_size=5).state.position == 5


def test_auto_load_state_ignores_an_unverifiable_state(
    data_file: Path,
    make_reader: Callable[..., PReader],
    append: Callable[[Path, bytes], None],
) -> None:
    reader = make_reader(auto_load_state=True)
    iterator = reader.chunks(data_file, chunk_size=5)

    next(iterator)

    iterator.state.save()

    append(data_file, b"tampered")

    assert reader.chunks(data_file, chunk_size=5).state.position == 0


def test_auto_load_state_resumes_stale_state_without_verification(
    data_file: Path,
    make_reader: Callable[..., PReader],
    append: Callable[[Path, bytes], None],
) -> None:
    reader = make_reader(auto_load_state=True, verify_state=False)
    iterator = reader.chunks(data_file, chunk_size=5)

    next(iterator)

    iterator.state.save()

    append(data_file, b"tampered")

    assert reader.chunks(data_file, chunk_size=5).state.position == 5


def test_auto_load_state_disabled_ignores_existing_state(
    reader: PReader, tmp_file: Path, consume: Callable[..., Any]
) -> None:
    consume(reader.chunks(tmp_file, chunk_size=3)).state.save()

    assert reader.chunks(tmp_file, chunk_size=3).state.position == 0


def test_state_name_change_creates_orphaned_state(
    reader: PReader, tmp_file: Path
) -> None:
    reader.chunks(tmp_file, state="job-old").state.save()

    new_state = reader.chunks(tmp_file, state="job-new").state

    assert new_state.position == 0
    assert "job-old" in reader.states


@pytest.mark.parametrize("state", [123, [], True], ids=["int", "list", "bool"])
def test_raises_when_state_has_an_unsupported_type(
    reader: PReader, data_file: Path, state: State
) -> None:
    with pytest.raises(TypeError, match="state must be None"):
        reader.chunks(data_file, state=state)


@pytest.mark.parametrize(
    ("name", "message"), TEST_UNSAFE_STATE_NAMES.items(), ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_unsafe_name_defers_rejection_to_save(
    reader: PReader, tmp_file: Path, name: str, message: str
) -> None:
    iterator = reader.chunks(tmp_file, state=name)
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
    iterator = reader.chunks(tmp_file, state=name)
    assert iterator.state.position == 0

    with pytest.raises(StateError, match="path escapes root"):
        iterator.state.save()


def test_raises_when_state_object_file_argument_mismatches(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    tracked = make_file(TEST_ALPHABET, name="tracked.bin")
    untracked = make_file(TEST_ALPHABET, name="untracked.bin")

    state = reader.chunks(tracked, state=TEST_STATE_NAME).state
    state.save()

    with pytest.raises(StateError, match=r"file path mismatch .*resync"):
        reader.chunks(untracked, state=state)


def test_verify_state_disabled_skips_the_mismatch_check(
    make_file: Callable[..., Path], make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(verify_state=False)
    tracked = make_file(TEST_ALPHABET, name="tracked.bin")
    untracked = make_file(TEST_ALPHABET.upper(), name="untracked.bin")

    state = reader.chunks(tracked, state=TEST_STATE_NAME).state
    state.save()

    resumed = reader.chunks(untracked, state=state)

    assert resumed.state.file.path == tracked
    assert b"".join(resumed) == TEST_ALPHABET


def test_verify_state_disabled_skips_verification(
    data_file: Path,
    make_reader: Callable[..., PReader],
    append: Callable[[Path, bytes], None],
) -> None:
    reader = make_reader()
    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3)

    next(iterator)

    state = iterator.state
    state.save()

    append(data_file, b"more")

    lenient_reader = make_reader(verify_state=False)
    resumed = lenient_reader.chunks(data_file, state=state, chunk_size=3)

    assert resumed.state.position == state.position


def test_raises_when_file_deleted_and_verify_disabled(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(verify_state=False)

    state = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3).state
    state.save()

    data_file.unlink()

    with pytest.raises(FileNotFoundError):
        reader.chunks(data_file, state=state, chunk_size=3)


def test_raises_when_the_tracked_file_is_deleted(
    make_file: Callable[..., Path], make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(verify_state=False)
    tracked = make_file(TEST_ALPHABET, name="tracked.bin")
    untracked = make_file(TEST_ALPHABET, name="untracked.bin")

    state = reader.chunks(tracked, state=TEST_STATE_NAME).state

    state.save()
    tracked.unlink()

    with pytest.raises(FileNotFoundError):
        reader.chunks(untracked, state=state)


def test_resync_allows_resuming_moved_file(
    reader: PReader, tmp_path: Path, data_file: Path, consume: Callable[..., Any]
) -> None:
    state = consume(
        reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=5), 1
    ).state
    state.save()

    moved = data_file.rename(tmp_path / "moved.bin")

    with pytest.raises(StateError, match="resync"):
        reader.chunks(moved, state=state)

    resynced = state.resync(moved)
    resumed = reader.chunks(moved, state=resynced, chunk_size=5)

    assert resumed.state.file.path == moved
    assert b"".join(resumed) == TEST_ALPHABET[5:]


def test_resync_allows_resuming_grown_file(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    state = consume(reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=5)).state
    state.save()

    append(data_file, b"more")

    with pytest.raises(StateError, match="size mismatch"):
        reader.chunks(data_file, state=state)

    resynced = state.resync(data_file)
    resumed = reader.chunks(data_file, state=resynced, chunk_size=5)

    assert resynced.file.size == len(TEST_ALPHABET) + len(b"more")
    assert b"".join(resumed) == b"more"


def test_reusing_state_name_for_different_file_reads_fresh_file(
    make_file: Callable[..., Path], make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_load_state=True)

    file_a = make_file(b"foo", name="data-1.bin")
    file_b = make_file(b"bar", name="data-2.bin")

    reader.chunks(file_a, state="shared-name").state.save()

    reused = reader.chunks(file_b, state="shared-name")

    assert reused.state.file.path == file_b
    assert next(reused) == b"bar"

    reused.state.save()

    assert reader.states["shared-name"].file.path == file_b


def test_auto_name_changes_when_file_moves(
    reader: PReader, tmp_path: Path, data_file: Path
) -> None:
    iterator = reader.chunks(data_file)

    next(iterator)

    iterator.state.save()

    moved = data_file.rename(tmp_path / "moved.bin")

    assert reader.chunks(moved).state.position == 0


def test_raises_when_resumed_after_file_grows(
    reader: PReader,
    data_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    state = consume(reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=5)).state
    state.save()

    append(data_file, b"more")

    with pytest.raises(StateError, match="size mismatch"):
        reader.chunks(data_file, state=state)


def test_recorded_file_size_never_refreshes_after_file_grows(
    data_file: Path,
    make_reader: Callable[..., PReader],
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    reader = make_reader(verify_state=False)

    state = consume(reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=5)).state
    state.save()

    append(data_file, b"more")

    options = IteratorOptions(end=100)
    resumed = reader.chunks(data_file, state=state, options=options)

    assert list(resumed) == []
    assert resumed.state.file.size == len(TEST_ALPHABET)


def test_clear_does_not_affect_live_iterator(reader: PReader, data_file: Path) -> None:
    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=5)

    next(iterator)

    iterator.state.save()
    reader.states.clear()

    assert TEST_STATE_NAME not in reader.states

    next(iterator)

    assert iterator.state.position == 10
    assert (
        reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=5).state.position
        == 0
    )


def test_raises_when_reading_a_directory(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(verify_state=False)
    state = reader.chunks(data_file, state=TEST_STATE_NAME).state
    state.save()

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises(OSError, match=r"os error"):
        list(reader.chunks(data_file, state=state))


def test_raises_when_resumed_file_replaced_by_directory(
    reader: PReader, data_file: Path
) -> None:
    state = reader.chunks(data_file, state=TEST_STATE_NAME).state
    state.save()

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises(StateError, match="io failed"):
        reader.chunks(data_file, state=state)


@pytest.mark.usefixtures("requires_symlinks")
def test_two_symlinks_to_same_target_share_auto_name(
    reader: PReader, tmp_path: Path, make_file: Callable[..., Path]
) -> None:
    real = make_file(TEST_ALPHABET, name="real.bin")

    link1 = tmp_path / "link-1.bin"
    link2 = tmp_path / "link-2.bin"

    link1.symlink_to(real)
    link2.symlink_to(real)

    assert reader.chunks(link1).state.name == reader.chunks(link2).state.name


def test_state_dir_change_creates_fresh_state(
    data_file: Path, make_reader: Callable[..., PReader], tmp_path: Path
) -> None:
    reader = make_reader()

    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=5)

    next(iterator)

    iterator.state.save()

    other_reader = PReader(config=Config(state_dir=tmp_path / "other-preader"))

    assert (
        other_reader.chunks(
            data_file, state=TEST_STATE_NAME, chunk_size=5
        ).state.position
        == 0
    )


def test_state_object_keeps_its_own_state_dir(
    data_file: Path, make_reader: Callable[..., PReader], tmp_path: Path
) -> None:
    owner = make_reader(auto_save_state=True)
    state = owner.bytes(data_file, state=TEST_STATE_NAME).state

    config = Config(
        state_dir=tmp_path / "other-preader", auto_save_state=True, verify_state=False
    )
    reader = PReader(config=config)

    list(reader.chunks(data_file, state=state))

    assert (owner.config.state_dir / f"{TEST_STATE_NAME}.state.json").is_file()
    assert not (config.state_dir / f"{TEST_STATE_NAME}.state.json").exists()


def test_auto_load_state_ignores_a_corrupt_payload(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_load_state=True)
    iterator = reader.chunks(data_file, chunk_size=3)

    next(iterator)

    state_path = iterator.state.save()
    state_path.write_text("not valid json")

    assert reader.chunks(data_file, chunk_size=3).state.position == 0


def test_auto_load_state_ignores_an_incomplete_payload(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_load_state=True)
    iterator = reader.chunks(data_file, chunk_size=3)

    next(iterator)

    state_path = iterator.state.save()
    payload = json.loads(state_path.read_text())

    del payload["timestamps"]

    state_path.write_text(json.dumps(payload))

    assert reader.chunks(data_file, chunk_size=3).state.position == 0


def test_second_iteration_yields_nothing(reader: PReader, data_file: Path) -> None:
    iterator = reader.chunks(data_file, chunk_size=3)

    list(iterator)

    assert list(iterator) == []


def test_drop_does_not_save_without_progress(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True)
    iterator = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3)

    del iterator

    assert TEST_STATE_NAME not in reader.states


def test_default_chunk_size_is_1024(reader: PReader, data_file: Path) -> None:
    assert reader.chunks(data_file).chunk_size == 1024


def test_whole_file_chunk_size_yields_one_chunk(
    reader: PReader, data_file: Path
) -> None:
    size = len(data_file.read_bytes())

    assert list(reader.chunks(data_file, chunk_size=size)) == [TEST_ALPHABET]


def test_start_is_not_realigned_to_the_chunk_grid(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(start=7)

    assert list(reader.chunks(data_file, chunk_size=3, options=options)) == [
        TEST_ALPHABET[i : i + 3] for i in range(7, len(TEST_ALPHABET), 3)
    ]


def test_skip_saturates_when_multiplied_by_the_chunk_size(
    reader: PReader, data_file: Path
) -> None:
    options = IteratorOptions(skip=2**32)

    assert list(reader.chunks(data_file, chunk_size=2**32, options=options)) == []


def test_drop_warns_on_stderr_when_saving_fails(
    data_file: Path,
    make_reader: Callable[..., PReader],
    capfd: pytest.CaptureFixture[str],
) -> None:
    reader = make_reader(auto_save_state=True)
    iterator = reader.chunks(data_file, chunk_size=3, state="../../etc/passwd")

    next(iterator)

    del iterator

    assert "preader: save failed" in capfd.readouterr().err


def test_chunk_size_of_one_yields_single_bytes(
    reader: PReader, data_file: Path
) -> None:
    chunks = list(reader.chunks(data_file, chunk_size=1))

    assert chunks == [bytes([value]) for value in TEST_ALPHABET]
    assert chunks == list(reader.bytes(data_file))


def test_threshold_autosave_records_every_item_boundary(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=10)
    saved: list[int] = []

    for _ in reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3):
        if TEST_STATE_NAME not in reader.states:
            continue

        position = reader.states[TEST_STATE_NAME].position

        if not saved or saved[-1] != position:
            saved.append(position)

    assert saved == [12, 24]
    assert reader.states[TEST_STATE_NAME].position == len(TEST_ALPHABET)


def test_file_growth_during_iteration_is_ignored(
    reader: PReader,
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
) -> None:
    path = make_file(b"abcdef")
    iterator = reader.chunks(path, chunk_size=3)

    next(iterator)

    append(path, b"more")

    assert list(iterator) == [b"def"]


def test_resume_applies_the_limit_again(
    data_file: Path, make_reader: Callable[..., PReader]
) -> None:
    reader = make_reader(auto_load_state=True)
    options = IteratorOptions(limit=2)
    iterator = reader.chunks(data_file, options=options, chunk_size=3)

    list(iterator)

    iterator.state.save()

    assert len(list(reader.chunks(data_file, options=options, chunk_size=3))) == 2


def test_two_iterators_with_the_same_name_advance_independently(
    reader: PReader, data_file: Path
) -> None:
    first = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3)
    second = reader.chunks(data_file, state=TEST_STATE_NAME, chunk_size=3)

    next(first)

    assert first.state.position == 3
    assert second.state.position == 0


def test_yields_every_byte_value(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(bytes(range(256)))

    assert list(reader.chunks(path, chunk_size=1)) == [
        bytes([value]) for value in range(256)
    ]


def test_drop_partial_reads_past_a_buffer_refill(
    make_reader: Callable[..., PReader], make_file: Callable[..., Path]
) -> None:
    reader = make_reader(buffer_capacity=64)
    content = TEST_ALPHABET * 20
    chunks = reader.chunks(make_file(content), chunk_size=7, drop_partial=True)

    assert b"".join(chunks) == content[: len(content) - len(content) % 7]


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


def test_percent_tracks_the_position(reader: PReader, data_file: Path) -> None:
    iterator = reader.chunks(data_file, chunk_size=7)
    size = len(data_file.read_bytes())

    assert iterator.percent() == 0.0

    for _ in iterator:
        assert iterator.percent() == pytest.approx(iterator.state.position / size * 100)

    assert iterator.percent() == 100.0


def test_iteration_survives_the_file_being_deleted(
    make_reader: Callable[..., PReader], data_file: Path
) -> None:
    reader = make_reader(buffer_capacity=1)
    expected = list(reader.chunks(data_file, chunk_size=3))
    iterator = reader.chunks(data_file, chunk_size=3)

    first = next(iterator)
    data_file.unlink()

    assert [first, *iterator] == expected
    assert iterator.state.position == len(TEST_ALPHABET)


def test_iteration_stops_at_a_truncation(
    make_reader: Callable[..., PReader],
    data_file: Path,
    truncate: Callable[[Path, int], None],
) -> None:
    reader = make_reader(buffer_capacity=1)
    iterator = reader.chunks(data_file, chunk_size=3)

    next(iterator)

    truncate(data_file, 10)

    list(iterator)

    assert iterator.state.position == 10
