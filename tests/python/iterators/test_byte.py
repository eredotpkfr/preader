import json

import pytest

from preader import Config, IteratorOptions, PReader, StateError

from constants import TEST_ALPHABET, TEST_STATE_NAME, TEST_UNSAFE_STATE_NAMES, TEST_UNSAFE_STATE_NAME_IDS


@pytest.fixture
def data_file(make_file):
    return make_file(TEST_ALPHABET)


@pytest.mark.parametrize(
    ("options", "expected"),
    [
        (IteratorOptions(start=5), TEST_ALPHABET[5:]),
        (IteratorOptions(end=5), TEST_ALPHABET[:5]),
        (IteratorOptions(skip=3), TEST_ALPHABET[3:]),
        (IteratorOptions(start=5, skip=3), TEST_ALPHABET[8:]),
        (IteratorOptions(start=10, limit=3), TEST_ALPHABET[10:13]),
        (IteratorOptions(start=5, end=10), TEST_ALPHABET[5:10]),
    ],
    ids=[
        "start_skips_to_position",
        "end_stops_before_position",
        "skip_skips_items",
        "start_and_skip_combine",
        "limit_caps_yielded_items",
        "start_and_end_define_window",
    ],
)
def test_options_narrow_the_output(reader, data_file, options, expected):
    assert b"".join(reader.bytes(data_file, options=options)) == expected


@pytest.mark.parametrize(
    ("options", "expected_length"),
    [(IteratorOptions(end=5, limit=100), 5), (IteratorOptions(end=100, limit=2), 2)],
    ids=["end_is_tighter", "limit_is_tighter"],
)
def test_options_end_and_limit_whichever_is_tighter_wins(reader, data_file, options, expected_length):
    assert b"".join(reader.bytes(data_file, options=options)) == TEST_ALPHABET[:expected_length]


@pytest.mark.parametrize("buffer_capacity", (0, 2), ids=["zero", "tiny"])
def test_buffer_capacity_smaller_than_the_file(data_file, make_reader, buffer_capacity):
    reader = make_reader(buffer_capacity=buffer_capacity)

    assert b"".join(reader.bytes(data_file)) == TEST_ALPHABET


def test_resume_ignores_options_when_already_past_start(reader, data_file, consume):
    options = IteratorOptions(start=5)
    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)

    state = consume(iterator, 15).state
    state.save()

    resumed = reader.bytes(data_file, state=state, options=options)

    assert b"".join(resumed) == TEST_ALPHABET[15:]


def test_threshold_autosave_triggers_mid_iteration(data_file, make_reader, consume):
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=10)
    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)

    consume(iterator, 10)

    assert reader.states[TEST_STATE_NAME].position == 10


def test_zero_threshold_saves_only_at_finalize(data_file, make_reader, consume):
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=0)
    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)

    consume(iterator, 3)

    position = iterator.state.position

    assert TEST_STATE_NAME not in reader.states

    del iterator

    assert reader.states[TEST_STATE_NAME].position == position


def test_drop_saves_partial_progress(data_file, make_reader, consume):
    reader = make_reader(auto_save_state=True)
    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)

    consume(iterator, 5)

    del iterator

    assert reader.states[TEST_STATE_NAME].position == 5


@pytest.mark.parametrize(
    "options",
    [
        IteratorOptions(start=1000),
        IteratorOptions(end=0),
        IteratorOptions(limit=0),
        IteratorOptions(skip=1000),
    ],
    ids=["start_beyond_size", "end_zero", "limit_zero", "skip_beyond_end"],
)
def test_boundary_options_yield_nothing(reader, tmp_file, options):
    assert list(reader.bytes(tmp_file, options=options)) == []


def test_resume_on_fully_consumed_file_yields_nothing(reader, tmp_file, consume):
    iterator = reader.bytes(tmp_file, state=TEST_STATE_NAME)

    state = consume(iterator).state
    state.save()

    resumed = reader.bytes(tmp_file, state=state)

    assert list(resumed) == []
    assert resumed.percent() == 100.0


def test_verify_state_disabled_skips_verification(data_file, make_reader, append):
    reader = make_reader()
    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)

    next(iterator)

    state = iterator.state
    state.save()

    append(data_file, b"more")

    lenient_reader = make_reader(verify_state=False)
    resumed = lenient_reader.bytes(data_file, state=state)

    assert resumed.state.position == state.position


def test_raises_when_file_deleted_and_verify_disabled(data_file, make_reader):
    reader = make_reader(verify_state=False)

    state = reader.bytes(data_file, state=TEST_STATE_NAME).state
    state.save()

    data_file.unlink()

    with pytest.raises(FileNotFoundError):
        reader.bytes(data_file, state=state)


def test_raises_when_state_object_file_argument_mismatches(reader, make_file):
    tracked = make_file(b"foo", name="tracked.bin")
    untracked = make_file(b"foo", name="untracked.bin")

    state = reader.bytes(tracked, state=TEST_STATE_NAME).state
    state.save()

    with pytest.raises(StateError, match="resync"):
        reader.bytes(untracked, state=state)


def test_verify_state_disabled_skips_the_mismatch_check(make_file, make_reader):
    reader = make_reader(verify_state=False)
    tracked = make_file(b"foo", name="tracked.bin")
    untracked = make_file(b"bar", name="untracked.bin")

    state = reader.bytes(tracked, state=TEST_STATE_NAME).state
    state.save()

    resumed = reader.bytes(untracked, state=state)

    assert resumed.state.file.path == tracked
    assert b"".join(resumed) == b"foo"


def test_resync_allows_resuming_moved_file(reader, tmp_path, data_file, consume):
    state = consume(reader.bytes(data_file, state=TEST_STATE_NAME), 1).state
    state.save()

    moved = data_file.rename(tmp_path / "moved.bin")

    with pytest.raises(StateError, match="resync"):
        reader.bytes(moved, state=state)

    resynced = state.resync(moved)
    resumed = reader.bytes(moved, state=resynced)

    assert resumed.state.file.path == moved
    assert b"".join(resumed) == TEST_ALPHABET[1:]


def test_resync_allows_resuming_grown_file(reader, data_file, consume, append):
    state = consume(reader.bytes(data_file, state=TEST_STATE_NAME)).state
    state.save()

    append(data_file, b"more")

    with pytest.raises(StateError, match="size mismatch"):
        reader.bytes(data_file, state=state)

    resynced = state.resync(data_file)
    resumed = reader.bytes(data_file, state=resynced)

    assert resynced.file.size == len(TEST_ALPHABET) + 4
    assert b"".join(resumed) == b"more"


def test_auto_load_state_disabled_ignores_existing_state(reader, tmp_file, consume):
    consume(reader.bytes(tmp_file)).state.save()

    assert reader.bytes(tmp_file).state.position == 0


@pytest.mark.parametrize("state", [123, [], True], ids=["int", "list", "bool"])
def test_raises_when_state_has_an_unsupported_type(reader, data_file, state):
    with pytest.raises(TypeError, match="state must be None"):
        reader.bytes(data_file, state=state)


@pytest.mark.parametrize(
    ("name", "message"), TEST_UNSAFE_STATE_NAMES.items(), ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_unsafe_name_defers_rejection_to_save(reader, tmp_file, name, message):
    iterator = reader.bytes(tmp_file, state=name)
    assert iterator.state.position == 0

    with pytest.raises(StateError, match=message):
        iterator.state.save()


def test_drop_does_not_save_when_auto_save_disabled(reader, tmp_file):
    iterator = reader.bytes(tmp_file, state=TEST_STATE_NAME)

    next(iterator)

    del iterator

    assert TEST_STATE_NAME not in reader.states


def test_extra_next_after_exhaustion_does_not_resave(data_file, make_reader, consume):
    reader = make_reader(auto_save_state=True)

    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)
    consume(iterator)

    mtime_before = reader.states[TEST_STATE_NAME].path.stat().st_mtime

    with pytest.raises(StopIteration):
        next(iterator)

    assert reader.states[TEST_STATE_NAME].path.stat().st_mtime == mtime_before


def test_autosave_error_propagates_from_unbound_iteration(tmp_path, data_file):
    blocking_file = tmp_path / "preader"
    blocking_file.write_bytes(b"foo")

    config = Config(state_dir=blocking_file, auto_save_state=True, auto_save_state_bytes=5)
    reader = PReader(config=config)

    with pytest.raises(StateError, match="io failed"):
        for _ in reader.bytes(data_file, state=TEST_STATE_NAME):
            pass


def test_byte_iterator_repr(reader, tmp_file, reindent, expected_repr):
    iterator = reader.bytes(tmp_file)

    assert repr(iterator) == expected_repr(
        "ByteIterator",
        state=reindent(repr(iterator.state), 2),
    )


def test_auto_load_state_resumes_previous_position(data_file, make_reader):
    reader = make_reader(auto_load_state=True)
    iterator = reader.bytes(data_file)

    next(iterator)

    iterator.state.save()

    assert reader.bytes(data_file).state.position == 1


def test_auto_load_state_ignores_an_unverifiable_state(data_file, make_reader, append):
    reader = make_reader(auto_load_state=True)
    iterator = reader.bytes(data_file)

    next(iterator)

    iterator.state.save()

    append(data_file, b"tampered")

    assert reader.bytes(data_file).state.position == 0


def test_auto_load_state_resumes_stale_state_without_verification(data_file, make_reader, append):
    reader = make_reader(auto_load_state=True, verify_state=False)
    iterator = reader.bytes(data_file)

    next(iterator)

    iterator.state.save()

    append(data_file, b"tampered")

    assert reader.bytes(data_file).state.position == 1


def test_state_name_change_creates_orphaned_state(reader, tmp_file):
    reader.bytes(tmp_file, state="job-old").state.save()

    new_state = reader.bytes(tmp_file, state="job-new").state

    assert new_state.position == 0
    assert "job-old" in reader.states


def test_reusing_state_name_for_different_file_reads_fresh_file(make_file, make_reader):
    reader = make_reader(auto_load_state=True)

    file_a = make_file(b"foo", name="data-1.bin")
    file_b = make_file(b"bar", name="data-2.bin")

    reader.bytes(file_a, state="shared-name").state.save()

    reused = reader.bytes(file_b, state="shared-name")

    assert reused.state.file.path == file_b
    assert next(reused) == b"b"

    reused.state.save()

    assert reader.states["shared-name"].file.path == file_b


def test_auto_name_changes_when_file_moves(reader, tmp_path, data_file):
    iterator = reader.bytes(data_file)

    next(iterator)

    iterator.state.save()

    moved = data_file.rename(tmp_path / "moved.bin")

    assert reader.bytes(moved).state.position == 0


def test_raises_when_resumed_after_file_grows(reader, data_file, consume, append):
    state = consume(reader.bytes(data_file, state=TEST_STATE_NAME)).state
    state.save()

    append(data_file, b"more")

    with pytest.raises(StateError, match="size mismatch"):
        reader.bytes(data_file, state=state)


def test_recorded_file_size_never_refreshes_after_file_grows(data_file, make_reader, consume, append):
    reader = make_reader(verify_state=False)

    state = consume(reader.bytes(data_file, state=TEST_STATE_NAME)).state
    state.save()

    append(data_file, b"more")

    options = IteratorOptions(end=100)
    resumed = reader.bytes(data_file, state=state, options=options)

    assert list(resumed) == []
    assert resumed.state.file.size == len(TEST_ALPHABET)


def test_clear_does_not_affect_live_iterator(reader, tmp_file):
    iterator = reader.bytes(tmp_file, state=TEST_STATE_NAME)

    next(iterator)

    iterator.state.save()
    reader.states.clear()

    assert TEST_STATE_NAME not in reader.states

    next(iterator)

    assert iterator.state.position == 2
    assert reader.bytes(tmp_file, state=TEST_STATE_NAME).state.position == 0


def test_resume_with_smaller_threshold_saves_early(data_file, make_reader, consume):
    first_reader = make_reader(auto_save_state=True, auto_save_state_bytes=1000)
    state = consume(first_reader.bytes(data_file, state=TEST_STATE_NAME), 7).state
    state.save()

    second_reader = make_reader(auto_save_state=True, auto_save_state_bytes=3)
    consume(second_reader.bytes(data_file, state=state), 2)

    assert second_reader.states[TEST_STATE_NAME].position == 9


def test_raises_when_resumed_file_replaced_by_directory(reader, data_file):
    state = reader.bytes(data_file, state=TEST_STATE_NAME).state
    state.save()

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises(StateError, match="Is a directory"):
        reader.bytes(data_file, state=state)


def test_two_symlinks_to_same_target_share_auto_name(reader, tmp_path, make_file):
    real = make_file(TEST_ALPHABET, name="real.bin")

    link1 = tmp_path / "link-1.bin"
    link2 = tmp_path / "link-2.bin"

    link1.symlink_to(real)
    link2.symlink_to(real)

    assert reader.bytes(link1).state.name == reader.bytes(link2).state.name


def test_state_dir_change_creates_fresh_state(data_file, make_reader, tmp_path):
    reader = make_reader()

    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)

    next(iterator)

    iterator.state.save()

    config = Config(state_dir=tmp_path / "other-preader")
    other_reader = PReader(config=config)

    assert other_reader.bytes(data_file, state=TEST_STATE_NAME).state.position == 0


def test_auto_load_state_ignores_a_corrupt_payload(data_file, make_reader):
    reader = make_reader(auto_load_state=True)
    iterator = reader.bytes(data_file)

    next(iterator)

    state_path = iterator.state.save()
    state_path.write_text("{ not valid json")

    assert reader.bytes(data_file).state.position == 0


def test_auto_load_state_ignores_an_incomplete_payload(data_file, make_reader):
    reader = make_reader(auto_load_state=True)
    iterator = reader.bytes(data_file)

    next(iterator)

    state_path = iterator.state.save()
    payload = json.loads(state_path.read_text())

    del payload["timestamps"]

    state_path.write_text(json.dumps(payload))

    assert reader.bytes(data_file).state.position == 0


def test_second_iteration_yields_nothing(reader, data_file):
    iterator = reader.bytes(data_file)

    list(iterator)

    assert list(iterator) == []


def test_drop_does_not_save_without_progress(data_file, make_reader):
    reader = make_reader(auto_save_state=True)
    iterator = reader.bytes(data_file, state=TEST_STATE_NAME)

    del iterator

    assert TEST_STATE_NAME not in reader.states


def test_resume_applies_the_limit_again(data_file, make_reader):
    reader = make_reader(auto_load_state=True)
    options = IteratorOptions(limit=3)
    iterator = reader.bytes(data_file, options=options)

    list(iterator)

    iterator.state.save()

    assert len(list(reader.bytes(data_file, options=options))) == 3


def test_file_growth_during_iteration_is_ignored(reader, make_file, append):
    path = make_file(b"abc")
    iterator = reader.bytes(path)

    next(iterator)

    append(path, b"more")

    assert list(iterator) == [b"b", b"c"]


def test_yields_every_byte_value(reader, make_file):
    path = make_file(bytes(range(256)))

    assert list(reader.bytes(path)) == [bytes([value]) for value in range(256)]


def test_two_iterators_with_the_same_name_advance_independently(reader, data_file):
    first = reader.bytes(data_file, state=TEST_STATE_NAME)
    second = reader.bytes(data_file, state=TEST_STATE_NAME)

    next(first)

    assert first.state.position == 1
    assert second.state.position == 0


def test_drop_warns_on_stderr_when_saving_fails(data_file, make_reader, capfd):
    reader = make_reader(auto_save_state=True)
    iterator = reader.bytes(data_file, state="../../etc/passwd")

    next(iterator)

    del iterator

    assert "preader: save failed" in capfd.readouterr().err


def test_threshold_autosave_records_every_item_boundary(data_file, make_reader):
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=10)
    saved = []

    for _ in reader.bytes(data_file, state=TEST_STATE_NAME):
        if TEST_STATE_NAME not in reader.states:
            continue

        position = reader.states[TEST_STATE_NAME].position

        if not saved or saved[-1] != position:
            saved.append(position)

    assert saved == [10, 20]
    assert reader.states[TEST_STATE_NAME].position == len(TEST_ALPHABET)


def test_percent_tracks_the_position(reader, data_file):
    iterator = reader.bytes(data_file)
    size = len(data_file.read_bytes())

    assert iterator.percent() == 0.0

    for _ in iterator:
        assert iterator.percent() == pytest.approx(iterator.state.position / size * 100)

    assert iterator.percent() == 100.0


def test_iteration_survives_the_file_being_deleted(make_reader, data_file):
    reader = make_reader(buffer_capacity=1)
    expected = list(reader.bytes(data_file))
    iterator = reader.bytes(data_file)

    first = next(iterator)
    data_file.unlink()

    assert [first, *iterator] == expected
    assert iterator.state.position == len(TEST_ALPHABET)


def test_iteration_stops_at_a_truncation(make_reader, data_file):
    reader = make_reader(buffer_capacity=1)
    iterator = reader.bytes(data_file)

    next(iterator)

    with open(data_file, "r+b") as file:
        file.truncate(10)

    list(iterator)

    assert iterator.state.position == 10
