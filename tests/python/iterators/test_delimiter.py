import json
import os

import pytest

from preader import Config, IteratorOptions, PReader, StateError

from constants import (
    TEST_DEFAULT_DELIMITER,
    TEST_STATE_NAME,
    TEST_UNSAFE_STATE_NAMES,
    TEST_UNSAFE_STATE_NAME_IDS,
    TEST_WINDOWS_UNSAFE_STATE_NAMES,
    TEST_WINDOWS_UNSAFE_STATE_NAME_IDS,
)

SEGMENTS = [f"seg-{i}" for i in range(10)]
DELIMITER_CONTENT = TEST_DEFAULT_DELIMITER.join(SEGMENTS).encode()
BLANK_SEGMENT_CONTENT = TEST_DEFAULT_DELIMITER.join(["seg-0", "", "seg-2", ""]).encode()


@pytest.fixture
def data_file(make_file):
    return make_file(DELIMITER_CONTENT)


@pytest.mark.parametrize(
    ("options", "expected"),
    [
        (IteratorOptions(start=12), SEGMENTS[2:]),
        (IteratorOptions(end=12), SEGMENTS[:2]),
        (IteratorOptions(skip=3), SEGMENTS[3:]),
        (IteratorOptions(skip=1, limit=2), SEGMENTS[1:3]),
    ],
    ids=[
        "start_skips_to_position",
        "end_stops_before_position",
        "skip_skips_items",
        "limit_caps_yielded_items",
    ],
)
def test_options_narrow_the_output(reader, data_file, options, expected):
    segments = list(reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, options=options))

    assert segments == [seg.encode() for seg in expected]


def test_end_yields_a_crossing_segment_whole(reader, data_file):
    options = IteratorOptions(end=15)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, options=options)

    assert list(iterator) == [segment.encode() for segment in SEGMENTS[:3]]
    assert iterator.state.position > options.end


def test_skip_stops_at_a_truncation(make_reader, data_file):
    reader = make_reader(verify_state=False, auto_load_state=True)
    reader.bytes(data_file, state=TEST_STATE_NAME).state.save()

    os.truncate(data_file, 8)

    options = IteratorOptions(skip=3)

    assert list(reader.delimiter(data_file, state=TEST_STATE_NAME, options=options, delimiter=TEST_DEFAULT_DELIMITER)) == []


def test_skip_past_the_end_yields_nothing(reader, data_file):
    options = IteratorOptions(skip=1000)

    assert list(reader.delimiter(data_file, options=options, delimiter=TEST_DEFAULT_DELIMITER)) == []


def test_align_start_skips_partial_segment(reader, data_file):
    options = IteratorOptions(start=14)

    segments = list(reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, options=options, align_start=True))
    expected = [seg.encode() for seg in SEGMENTS[3:]]

    assert segments == expected


def test_align_start_skips_nothing_when_already_aligned(reader, data_file):
    options = IteratorOptions(start=12)

    aligned = list(
        reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, options=options, align_start=True)
    )

    assert aligned == list(reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, options=options))
    assert aligned == [seg.encode() for seg in SEGMENTS[2:]]


def test_keeps_partial_segment_by_default(reader, data_file):
    options = IteratorOptions(start=14)

    segments = list(reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, options=options))
    expected = [b"g-2"] + [seg.encode() for seg in SEGMENTS[3:]]

    assert segments == expected


@pytest.mark.parametrize("buffer_capacity", (0, 2), ids=["zero", "tiny"])
def test_buffer_capacity_smaller_than_segment_length(data_file, make_reader, buffer_capacity):
    reader = make_reader(buffer_capacity=buffer_capacity)

    assert list(reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER)) == [seg.encode() for seg in SEGMENTS]


def test_no_delimiter_yields_single_segment(reader, make_file):
    content = DELIMITER_CONTENT.replace(TEST_DEFAULT_DELIMITER.encode(), b"")
    path = make_file(content)

    assert list(reader.delimiter(path, delimiter=TEST_DEFAULT_DELIMITER)) == [content]


def test_null_byte_delimiter_splits_the_content(reader, make_file):
    path = make_file(b"foo\x00bar\x00baz")

    assert list(reader.delimiter(path, delimiter="\x00")) == [b"foo", b"bar", b"baz"]


def test_resume_with_different_delimiter(reader, make_file, consume):
    content = DELIMITER_CONTENT + b";" + DELIMITER_CONTENT
    path = make_file(content)

    state = consume(reader.delimiter(path, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME), 1).state
    state.save()

    resumed = reader.delimiter(path, delimiter=";", state=state)

    rest_of_first_copy = TEST_DEFAULT_DELIMITER.join(SEGMENTS[1:]).encode()

    assert list(resumed) == [rest_of_first_copy, DELIMITER_CONTENT]


def test_resume_applies_skip_when_the_position_equals_the_start(reader, data_file, consume):
    saved = consume(reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME), 2).state
    saved.save()

    options = IteratorOptions(start=saved.position, skip=1)
    resumed = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=saved, options=options)

    assert list(resumed) == [segment.encode() for segment in SEGMENTS[3:]]


def test_resume_ignores_options_when_already_past_start(reader, data_file, consume):
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)
    options = IteratorOptions(start=5, skip=1)

    state = consume(iterator, 3).state
    state.save()

    resumed = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=state, options=options)
    expected = [seg.encode() for seg in SEGMENTS[3:]]

    assert list(resumed) == expected


def test_end_below_the_position_does_not_rewind_the_state(data_file, make_reader):
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=64, auto_load_state=True)

    list(reader.delimiter(
        data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME
    ))

    saved = reader.states[TEST_STATE_NAME].position

    assert saved == len(data_file.read_bytes())

    list(reader.delimiter(
        data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME, options=IteratorOptions(end=5)
    ))

    assert reader.states[TEST_STATE_NAME].position == saved


def test_resume_on_fully_consumed_file_yields_nothing(reader, tmp_file, consume):
    iterator = reader.delimiter(tmp_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)

    state = consume(iterator).state
    state.save()

    resumed = reader.delimiter(tmp_file, delimiter=TEST_DEFAULT_DELIMITER, state=state)

    assert list(resumed) == []
    assert resumed.percent() == 100.0


def test_threshold_autosave_triggers_mid_iteration(data_file, make_reader, consume):
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=12)

    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)
    consume(iterator, 2)

    assert reader.states[TEST_STATE_NAME].position == 12


def test_drop_saves_partial_progress(data_file, make_reader):
    reader = make_reader(auto_save_state=True)

    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)

    next(iterator)

    del iterator

    assert reader.states[TEST_STATE_NAME].position == 6


def test_drop_does_not_save_when_auto_save_disabled(reader, tmp_file):
    iterator = reader.delimiter(tmp_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)

    next(iterator)

    del iterator

    assert TEST_STATE_NAME not in reader.states


def test_zero_threshold_saves_only_at_finalize(data_file, make_reader, consume):
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=0)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)

    consume(iterator, 3)
    position = iterator.state.position

    assert TEST_STATE_NAME not in reader.states

    del iterator

    assert reader.states[TEST_STATE_NAME].position == position


def test_extra_next_after_exhaustion_does_not_resave(data_file, make_reader, consume):
    reader = make_reader(auto_save_state=True)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)

    consume(iterator)

    mtime_before = reader.states[TEST_STATE_NAME].path.stat().st_mtime

    with pytest.raises(StopIteration):
        next(iterator)

    assert reader.states[TEST_STATE_NAME].path.stat().st_mtime == mtime_before


def test_autosave_error_propagates_from_unbound_iteration(config, make_reader, data_file, capfd):
    config.state_dir.write_bytes(b"foo")

    reader = make_reader(auto_save_state=True, auto_save_state_bytes=5)

    with pytest.raises(StateError, match="io failed"):
        for _ in reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME):
            pass

    assert "preader: save failed" not in capfd.readouterr().err


def test_save_error_at_finalize_propagates(data_file, make_reader):
    reader = make_reader(auto_save_state=True)

    with pytest.raises(StateError, match="path escapes root"):
        for _ in reader.delimiter(data_file, state="../../etc/passwd", delimiter=TEST_DEFAULT_DELIMITER):
            pass


def test_save_error_propagates_after_a_truncation(data_file, make_reader):
    reader = make_reader(auto_save_state=True, buffer_capacity=1)
    iterator = reader.delimiter(
        data_file, state="../../etc/passwd", delimiter=TEST_DEFAULT_DELIMITER
    )

    next(iterator)

    with open(data_file, "r+b") as file:
        file.truncate(1)

    with pytest.raises(StateError, match="path escapes root"):
        list(iterator)


def test_save_error_propagates_while_skipping(data_file, make_reader):
    reader = make_reader(auto_save_state=True, buffer_capacity=1)
    options = IteratorOptions(skip=3)
    iterator = reader.delimiter(
        data_file,
        state="../../etc/passwd",
        delimiter=TEST_DEFAULT_DELIMITER,
        options=options,
    )

    with open(data_file, "r+b") as file:
        file.truncate(1)

    with pytest.raises(StateError, match="path escapes root"):
        list(iterator)


def test_save_error_propagates_on_a_skipped_item(data_file, make_reader):
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    options = IteratorOptions(skip=2)
    iterator = reader.delimiter(
        data_file,
        state="../../etc/passwd",
        delimiter=TEST_DEFAULT_DELIMITER,
        options=options,
    )

    with pytest.raises(StateError, match="path escapes root"):
        list(iterator)

    assert iterator.state.position == len(SEGMENTS[0]) + 1


def test_save_error_propagates_on_a_filtered_blank(make_file, make_reader):
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=1)
    path = make_file(TEST_DEFAULT_DELIMITER.encode() + b"foo")

    iterator = reader.delimiter(
            path,
            state="../../etc/passwd",
            delimiter=TEST_DEFAULT_DELIMITER,
            skip_empty=True,
        )

    with pytest.raises(StateError, match="path escapes root"):
        list(iterator)

    assert iterator.state.position == 1


def test_delimiter_iterator_repr(reader, tmp_file, reindent, expected_repr):
    iterator = reader.delimiter(tmp_file, delimiter=TEST_DEFAULT_DELIMITER, keep_delimiter=True, skip_empty=True)

    assert repr(iterator) == expected_repr(
        "DelimiterIterator",
        state=reindent(repr(iterator.state), 2),
        delimiter=iterator.delimiter,
        keep_delimiter=str(iterator.keep_delimiter).lower(),
        skip_empty=str(iterator.skip_empty).lower(),
        skip_remaining=iterator.skip_remaining,
    )


def test_auto_load_state_resumes_previous_position(data_file, make_reader):
    reader = make_reader(auto_load_state=True)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER)

    next(iterator)

    iterator.state.save()

    assert reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER).state.position == 6


def test_auto_load_state_ignores_an_unverifiable_state(data_file, make_reader, append):
    reader = make_reader(auto_load_state=True)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER)

    next(iterator)

    iterator.state.save()

    append(data_file, b"tampered")

    assert reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER).state.position == 0


def test_auto_load_state_resumes_stale_state_without_verification(data_file, make_reader, append):
    reader = make_reader(auto_load_state=True, verify_state=False)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER)

    next(iterator)

    iterator.state.save()

    append(data_file, b"tampered")

    assert reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER).state.position == 6


def test_auto_load_state_disabled_ignores_existing_state(reader, tmp_file, consume):
    consume(reader.delimiter(tmp_file, delimiter=TEST_DEFAULT_DELIMITER)).state.save()

    assert reader.delimiter(tmp_file, delimiter=TEST_DEFAULT_DELIMITER).state.position == 0


def test_state_name_change_creates_orphaned_state(reader, tmp_file):
    reader.delimiter(tmp_file, delimiter=TEST_DEFAULT_DELIMITER, state="job-old").state.save()

    new_state = reader.delimiter(tmp_file, delimiter=TEST_DEFAULT_DELIMITER, state="job-new").state

    assert new_state.position == 0
    assert "job-old" in reader.states


@pytest.mark.parametrize("state", [123, [], True], ids=["int", "list", "bool"])
def test_raises_when_state_has_an_unsupported_type(reader, data_file, state):
    with pytest.raises(TypeError, match="state must be None"):
        reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=state)


@pytest.mark.parametrize(
    ("name", "message"), TEST_UNSAFE_STATE_NAMES.items(), ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_unsafe_name_defers_rejection_to_save(reader, tmp_file, name, message):
    iterator = reader.delimiter(tmp_file, delimiter=TEST_DEFAULT_DELIMITER, state=name)
    assert iterator.state.position == 0

    with pytest.raises(StateError, match=message):
        iterator.state.save()


@pytest.mark.skipif(os.name != "nt", reason="Windows path syntax is only unsafe on Windows")
@pytest.mark.parametrize(
    "name", TEST_WINDOWS_UNSAFE_STATE_NAMES, ids=TEST_WINDOWS_UNSAFE_STATE_NAME_IDS
)
def test_unsafe_windows_name_defers_rejection_to_save(reader, tmp_file, name):
    iterator = reader.delimiter(tmp_file, delimiter=TEST_DEFAULT_DELIMITER, state=name)
    assert iterator.state.position == 0

    with pytest.raises(StateError, match="path escapes root"):
        iterator.state.save()


def test_raises_when_state_object_file_argument_mismatches(reader, make_file):
    tracked = make_file(DELIMITER_CONTENT, name="tracked.bin")
    untracked = make_file(DELIMITER_CONTENT, name="untracked.bin")

    state = reader.delimiter(tracked, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME).state
    state.save()

    with pytest.raises(StateError, match=r"file path mismatch .*resync"):
        reader.delimiter(untracked, delimiter=TEST_DEFAULT_DELIMITER, state=state)


def test_verify_state_disabled_skips_the_mismatch_check(make_file, make_reader):
    reader = make_reader(verify_state=False)
    tracked = make_file(DELIMITER_CONTENT, name="tracked.bin")
    untracked = make_file(DELIMITER_CONTENT.upper(), name="untracked.bin")

    state = reader.delimiter(tracked, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME).state
    state.save()

    resumed = reader.delimiter(untracked, delimiter=TEST_DEFAULT_DELIMITER, state=state)

    assert resumed.state.file.path == tracked
    assert list(resumed) == [seg.encode() for seg in SEGMENTS]


def test_verify_state_disabled_skips_verification(data_file, make_reader, append):
    reader = make_reader()
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)

    next(iterator)

    state = iterator.state
    state.save()

    append(data_file, b"more")

    lenient_reader = make_reader(verify_state=False)
    resumed = lenient_reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=state)

    assert resumed.state.position == state.position


def test_raises_when_file_deleted_and_verify_disabled(data_file, make_reader):
    reader = make_reader(verify_state=False)

    state = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME).state
    state.save()

    data_file.unlink()

    with pytest.raises(FileNotFoundError):
        reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=state)


def test_raises_when_the_tracked_file_is_deleted(make_file, make_reader):
    reader = make_reader(verify_state=False)
    tracked = make_file(DELIMITER_CONTENT, name="tracked.bin")
    untracked = make_file(DELIMITER_CONTENT, name="untracked.bin")

    state = reader.delimiter(tracked, state=TEST_STATE_NAME, delimiter=TEST_DEFAULT_DELIMITER).state

    state.save()
    tracked.unlink()

    with pytest.raises(FileNotFoundError):
        reader.delimiter(untracked, state=state, delimiter=TEST_DEFAULT_DELIMITER)


def test_resync_allows_resuming_moved_file(reader, tmp_path, data_file, consume):
    state = consume(reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME), 1).state
    state.save()

    moved = data_file.rename(tmp_path / "moved.bin")

    with pytest.raises(StateError, match="resync"):
        reader.delimiter(moved, delimiter=TEST_DEFAULT_DELIMITER, state=state)

    resynced = state.resync(moved)
    resumed = reader.delimiter(moved, delimiter=TEST_DEFAULT_DELIMITER, state=resynced)

    assert resumed.state.file.path == moved
    assert list(resumed) == [seg.encode() for seg in SEGMENTS[1:]]


def test_resync_allows_resuming_grown_file(reader, data_file, consume, append):
    state = consume(reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)).state
    state.save()

    append(data_file, b"more,")

    with pytest.raises(StateError, match="size mismatch"):
        reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=state)

    resynced = state.resync(data_file)
    resumed = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=resynced)

    assert resynced.file.size == len(DELIMITER_CONTENT) + len(b"more,")
    assert list(resumed) == [b"more"]


def test_reusing_state_name_for_different_file_reads_fresh_file(make_file, make_reader):
    reader = make_reader(auto_load_state=True)

    file_a = make_file(b"foo,", name="data-1.bin")
    file_b = make_file(b"bar,", name="data-2.bin")

    reader.delimiter(file_a, delimiter=TEST_DEFAULT_DELIMITER, state="shared-name").state.save()

    reused = reader.delimiter(file_b, delimiter=TEST_DEFAULT_DELIMITER, state="shared-name")

    assert reused.state.file.path == file_b
    assert next(reused) == b"bar"

    reused.state.save()

    assert reader.states["shared-name"].file.path == file_b


def test_auto_name_changes_when_file_moves(reader, tmp_path, data_file):
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER)

    next(iterator)

    iterator.state.save()

    moved = data_file.rename(tmp_path / "moved.bin")

    assert reader.delimiter(moved, delimiter=TEST_DEFAULT_DELIMITER).state.position == 0


def test_raises_when_resumed_after_file_grows(reader, data_file, consume, append):
    state = consume(reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)).state
    state.save()

    append(data_file, b"more,")

    with pytest.raises(StateError, match="size mismatch"):
        reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=state)


def test_recorded_file_size_never_refreshes_after_file_grows(data_file, make_reader, consume, append):
    reader = make_reader(verify_state=False)

    state = consume(reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)).state
    state.save()

    append(data_file, b"more,")

    options = IteratorOptions(end=100)
    resumed = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=state, options=options)

    assert list(resumed) == []
    assert resumed.state.file.size == len(DELIMITER_CONTENT)


def test_clear_does_not_affect_live_iterator(reader, data_file):
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)

    next(iterator)

    iterator.state.save()
    reader.states.clear()

    assert TEST_STATE_NAME not in reader.states

    next(iterator)

    assert iterator.state.position == 12
    assert reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME).state.position == 0


@pytest.mark.parametrize(
    "options",
    [IteratorOptions(), IteratorOptions(skip=1)],
    ids=["reading", "skipping"],
)
def test_raises_when_reading_a_directory(data_file, make_reader, options):
    reader = make_reader(verify_state=False)
    state = reader.delimiter(
        data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME
    ).state
    state.save()

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises(OSError):
        list(
            reader.delimiter(
                data_file, delimiter=TEST_DEFAULT_DELIMITER, state=state, options=options
            )
        )


def test_raises_when_aligning_on_a_directory(data_file, make_reader):
    reader = make_reader(verify_state=False)
    state = reader.delimiter(
        data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME
    ).state
    state.save()

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises(OSError):
        reader.delimiter(
            data_file,
            delimiter=TEST_DEFAULT_DELIMITER,
            state=state,
            options=IteratorOptions(start=5),
            align_start=True,
        )


def test_raises_when_resumed_file_replaced_by_directory(reader, data_file):
    state = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME).state
    state.save()

    data_file.unlink()
    data_file.mkdir()

    with pytest.raises(StateError, match="io failed"):
        reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=state)


@pytest.mark.usefixtures("requires_symlinks")
def test_two_symlinks_to_same_target_share_auto_name(reader, tmp_path, make_file):
    real = make_file(DELIMITER_CONTENT, name="real.bin")

    link1 = tmp_path / "link-1.bin"
    link2 = tmp_path / "link-2.bin"

    link1.symlink_to(real)
    link2.symlink_to(real)

    assert reader.delimiter(link1, delimiter=TEST_DEFAULT_DELIMITER).state.name == reader.delimiter(link2, delimiter=TEST_DEFAULT_DELIMITER).state.name


def test_state_dir_change_creates_fresh_state(data_file, make_reader, tmp_path):
    reader = make_reader()

    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME)

    next(iterator)

    iterator.state.save()

    other_reader = PReader(config=Config(state_dir=tmp_path / "other-preader"))

    assert other_reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=TEST_STATE_NAME).state.position == 0


def test_state_object_keeps_its_own_state_dir(data_file, make_reader, tmp_path):
    owner = make_reader(auto_save_state=True)
    state = owner.bytes(data_file, state=TEST_STATE_NAME).state

    config = Config(state_dir=tmp_path / "other-preader", auto_save_state=True, verify_state=False)
    reader = PReader(config=config)

    list(reader.delimiter(data_file, state=state, delimiter=TEST_DEFAULT_DELIMITER))

    assert (owner.config.state_dir / f"{TEST_STATE_NAME}.state.json").is_file()
    assert not (config.state_dir / f"{TEST_STATE_NAME}.state.json").exists()


def test_resume_ignores_align_start_and_skip(reader, data_file, consume):
    options = IteratorOptions(start=14)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, options=options, align_start=True, state=TEST_STATE_NAME)

    state = consume(iterator, 1).state
    state.save()

    resumed_options = IteratorOptions(start=0, skip=5)
    resumed = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state=state, options=resumed_options, align_start=True)
    expected = [seg.encode() for seg in SEGMENTS[4:]]

    assert list(resumed) == expected


def test_newline_delimiter_splits_lines(reader, make_file):
    path = make_file(b"foo\nbar\n")

    assert list(reader.delimiter(path, delimiter="\n")) == [b"foo", b"bar"]


def test_delimiter_only_file_yields_blank_segments(reader, make_file):
    path = make_file(b"|||")

    assert list(reader.delimiter(path, delimiter="|")) == [b"", b"", b""]


def test_delimiter_only_file_keeps_each_delimiter(reader, make_file):
    path = make_file(b"|||")

    assert list(reader.delimiter(path, delimiter="|", keep_delimiter=True)) == [b"|", b"|", b"|"]


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
    reader, make_file, align_start, keep_delimiter, skip_empty, expected
):
    path = make_file(BLANK_SEGMENT_CONTENT)
    options = IteratorOptions(start=3)

    items = reader.delimiter(
        path,
        options=options,
        delimiter=TEST_DEFAULT_DELIMITER,
        align_start=align_start,
        keep_delimiter=keep_delimiter,
        skip_empty=skip_empty,
    )

    assert list(items) == expected


def test_auto_load_state_ignores_a_corrupt_payload(data_file, make_reader):
    reader = make_reader(auto_load_state=True)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER)

    next(iterator)

    state_path = iterator.state.save()
    state_path.write_text("not valid json")

    assert reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER).state.position == 0


def test_auto_load_state_ignores_an_incomplete_payload(data_file, make_reader):
    reader = make_reader(auto_load_state=True)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER)

    next(iterator)

    state_path = iterator.state.save()
    payload = json.loads(state_path.read_text())

    del payload["timestamps"]

    state_path.write_text(json.dumps(payload))

    assert reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER).state.position == 0


def test_second_iteration_yields_nothing(reader, data_file):
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER)

    list(iterator)

    assert list(iterator) == []


def test_drop_does_not_save_without_progress(data_file, make_reader):
    reader = make_reader(auto_save_state=True)
    iterator = reader.delimiter(data_file, state=TEST_STATE_NAME, delimiter=TEST_DEFAULT_DELIMITER)

    del iterator

    assert TEST_STATE_NAME not in reader.states


def test_drop_warns_on_stderr_when_saving_fails(data_file, make_reader, capfd):
    reader = make_reader(auto_save_state=True)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, state="../../etc/passwd")

    next(iterator)

    del iterator

    assert "preader: save failed" in capfd.readouterr().err


def test_skip_counts_blank_items_before_skip_empty(reader, make_file):
    path = make_file(b"foo,,bar,baz")
    options = IteratorOptions(skip=1)

    first = reader.delimiter(path, delimiter=TEST_DEFAULT_DELIMITER, options=options)
    second = reader.delimiter(path, delimiter=TEST_DEFAULT_DELIMITER, options=options, skip_empty=True)

    assert list(first) == [b"", b"bar", b"baz"]
    assert list(second) == [b"bar", b"baz"]


def test_skip_beyond_the_item_count_yields_nothing(reader, data_file):
    options = IteratorOptions(skip=len(SEGMENTS) + 1)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, options=options)

    assert list(iterator) == []
    assert iterator.state.position == len(DELIMITER_CONTENT)


def test_align_start_and_skip_combine(reader, data_file):
    options = IteratorOptions(start=3, skip=1)

    aligned = list(
        reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, options=options, align_start=True)
    )
    unaligned = list(
        reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, options=options)
    )

    assert aligned == [segment.encode() for segment in SEGMENTS[2:]]
    assert unaligned == [segment.encode() for segment in SEGMENTS[1:]]


def test_keep_delimiter_does_not_change_the_position(reader, data_file, consume):
    kept = consume(
        reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, keep_delimiter=True)
    ).state.position
    stripped = consume(
        reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER, keep_delimiter=False)
    ).state.position

    assert kept == stripped == len(DELIMITER_CONTENT)


def test_threshold_autosave_records_every_item_boundary(data_file, make_reader):
    reader = make_reader(auto_save_state=True, auto_save_state_bytes=10)
    saved = []

    for _ in reader.delimiter(data_file, state=TEST_STATE_NAME, delimiter=TEST_DEFAULT_DELIMITER):
        if TEST_STATE_NAME not in reader.states:
            continue

        position = reader.states[TEST_STATE_NAME].position

        if not saved or saved[-1] != position:
            saved.append(position)

    assert saved == [12, 24, 36, 48, 59]
    assert reader.states[TEST_STATE_NAME].position == len(DELIMITER_CONTENT)


def test_file_growth_during_iteration_is_ignored(reader, make_file, append):
    path = make_file(b"foo,bar,")
    iterator = reader.delimiter(path, delimiter=TEST_DEFAULT_DELIMITER)

    next(iterator)

    append(path, b"more")

    assert list(iterator) == [b"bar"]


def test_resume_applies_the_limit_again(data_file, make_reader):
    reader = make_reader(auto_load_state=True)
    options = IteratorOptions(limit=2)
    iterator = reader.delimiter(data_file, options=options, delimiter=TEST_DEFAULT_DELIMITER)

    list(iterator)

    iterator.state.save()

    assert len(list(reader.delimiter(data_file, options=options, delimiter=TEST_DEFAULT_DELIMITER))) == 2


def test_two_iterators_with_the_same_name_advance_independently(reader, data_file):
    first = reader.delimiter(data_file, state=TEST_STATE_NAME, delimiter=TEST_DEFAULT_DELIMITER)
    second = reader.delimiter(data_file, state=TEST_STATE_NAME, delimiter=TEST_DEFAULT_DELIMITER)

    next(first)

    assert first.state.position == 6
    assert second.state.position == 0


def test_yields_every_byte_value(reader, make_file):
    content = bytes(range(256))
    path = make_file(content)

    segments = reader.delimiter(path, delimiter=TEST_DEFAULT_DELIMITER, keep_delimiter=True)

    assert b"".join(segments) == content


def test_percent_tracks_the_position(reader, data_file):
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER)
    size = len(data_file.read_bytes())

    assert iterator.percent() == 0.0

    for _ in iterator:
        assert iterator.percent() == pytest.approx(iterator.state.position / size * 100)

    assert iterator.percent() == 100.0


def test_iteration_survives_the_file_being_deleted(make_reader, data_file):
    reader = make_reader(buffer_capacity=1)
    expected = list(reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER))
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER)

    first = next(iterator)
    data_file.unlink()

    assert [first, *iterator] == expected
    assert iterator.state.position == len(DELIMITER_CONTENT)


def test_iteration_stops_at_a_truncation(make_reader, data_file):
    reader = make_reader(buffer_capacity=1)
    iterator = reader.delimiter(data_file, delimiter=TEST_DEFAULT_DELIMITER)

    next(iterator)

    with open(data_file, "r+b") as file:
        file.truncate(10)

    list(iterator)

    assert iterator.state.position == 10
