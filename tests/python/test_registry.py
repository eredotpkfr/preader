import json
import os

import pytest

from preader import PReader, StateError

from constants import (
    TEST_STATE_NAME,
    TEST_UNSAFE_STATE_NAMES,
    TEST_UNSAFE_STATE_NAME_IDS,
    TEST_WINDOWS_UNSAFE_STATE_NAMES,
    TEST_WINDOWS_UNSAFE_STATE_NAME_IDS,
)

OTHER_STATE_NAME = "job-2"


def test_len(reader, registry, tmp_file):
    assert len(registry) == 0

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert len(registry) == 1


def test_contains(reader, registry, tmp_file):
    assert TEST_STATE_NAME not in registry

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert TEST_STATE_NAME in registry


@pytest.mark.parametrize("name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS)
def test_contains_returns_false_when_name_is_unsafe(registry, name):
    assert name not in registry


def test_names(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=OTHER_STATE_NAME).state.save()

    assert set(registry.names()) == {TEST_STATE_NAME, OTHER_STATE_NAME}


def test_iter(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=OTHER_STATE_NAME).state.save()

    assert set(registry) == {TEST_STATE_NAME, OTHER_STATE_NAME}


def test_getitem_loads_saved_state(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    state = registry[TEST_STATE_NAME]

    assert state.name == TEST_STATE_NAME
    assert state.position == 0
    assert state.file.path == tmp_file


def test_getitem_accepts_an_already_suffixed_name(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    state = registry[f"{TEST_STATE_NAME}.state.json"]

    assert state.name == TEST_STATE_NAME
    assert state.file.path == tmp_file


def test_getitem_raises_when_missing(registry):
    with pytest.raises(KeyError):
        registry["missing"]


@pytest.mark.parametrize("name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS)
def test_getitem_raises_when_name_is_unsafe(registry, name):
    with pytest.raises(KeyError):
        registry[name]


def test_getitem_raises_when_state_is_a_directory(registry):
    registry.path(TEST_STATE_NAME).mkdir(parents=True)

    with pytest.raises(StateError, match="state not found"):
        registry[TEST_STATE_NAME]


def test_getitem_ignores_unknown_fields(make_reader, tmp_file):
    reader = make_reader(verify_state=False)
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    path = reader.states.path(TEST_STATE_NAME)
    payload = json.loads(path.read_text())

    payload["foo"] = 42
    path.write_text(json.dumps(payload))

    state = reader.states[TEST_STATE_NAME]

    assert state.name == TEST_STATE_NAME
    assert state.file.path == tmp_file


@pytest.mark.parametrize(
    "name",
    ["README.md", f"{TEST_STATE_NAME}.state.json.tmp"],
    ids=["unrelated_file", "temp_state_file"],
)
def test_names_ignores_non_state_files(reader, registry, tmp_file, name):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    registry.path(TEST_STATE_NAME).parent.joinpath(name).write_text("not a state")

    assert set(registry.names()) == {TEST_STATE_NAME}
    assert len(registry) == 1


def test_clear_keeps_non_state_files(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    unrelated = registry.path(TEST_STATE_NAME).parent / "README.md"
    unrelated.write_text("not a state")

    registry.clear()

    assert len(registry) == 0
    assert unrelated.exists()


def test_search_returns_nothing_without_a_match(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert list(registry.search("no-such-name")) == []


def test_find_returns_none_when_missing(registry):
    assert registry.find("missing") is None


@pytest.mark.parametrize("name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS)
def test_find_returns_none_when_name_is_unsafe(registry, name):
    assert registry.find(name) is None


def test_find_raises_when_state_is_corrupted(registry, reader, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    registry.path(TEST_STATE_NAME).write_text("not valid json")

    with pytest.raises(StateError, match="line 1 column"):
        registry.find(TEST_STATE_NAME)


def test_find_returns_state_when_present(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    state = registry.find(TEST_STATE_NAME)

    assert state.name == TEST_STATE_NAME
    assert state.position == 0
    assert state.file.path == tmp_file


def test_search_filters_by_pattern(reader, registry, tmp_file):
    reader.bytes(tmp_file, state="foo-1").state.save()
    reader.bytes(tmp_file, state="foo-2").state.save()
    reader.bytes(tmp_file, state="bar-1").state.save()

    assert set(registry.search("^foo")) == {"foo-1", "foo-2"}


def test_delitem_removes_saved_state(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    del registry[TEST_STATE_NAME]

    assert TEST_STATE_NAME not in registry


def test_delitem_raises_when_missing(registry):
    with pytest.raises(KeyError):
        del registry["missing"]


@pytest.mark.parametrize("name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS)
def test_delitem_raises_when_name_is_unsafe(registry, name):
    with pytest.raises(KeyError):
        del registry[name]


def test_path_accepts_an_already_suffixed_name(registry):
    assert registry.path(f"{TEST_STATE_NAME}.state.json") == registry.path(TEST_STATE_NAME)


@pytest.mark.parametrize(
    ("name", "message"), TEST_UNSAFE_STATE_NAMES.items(), ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_path_raises_when_name_is_unsafe(registry, name, message):
    with pytest.raises(StateError, match=message):
        registry.path(name)


@pytest.mark.skipif(os.name != "nt", reason="Windows path syntax is only unsafe on Windows")
@pytest.mark.parametrize(
    "name", TEST_WINDOWS_UNSAFE_STATE_NAMES, ids=TEST_WINDOWS_UNSAFE_STATE_NAME_IDS
)
def test_path_raises_when_a_windows_name_is_unsafe(registry, name):
    with pytest.raises(StateError, match="path escapes root"):
        registry.path(name)


def test_all_returns_every_saved_state(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=OTHER_STATE_NAME).state.save()

    states = {(state.name, state.position, state.file.path) for state in registry.all()}

    assert states == {
        (TEST_STATE_NAME, 0, tmp_file),
        (OTHER_STATE_NAME, 0, tmp_file),
    }


def test_clear_removes_saved_states(reader, registry, tmp_file):
    state1 = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    state2 = reader.bytes(tmp_file, state=OTHER_STATE_NAME).state

    state1.save()
    state2.save()

    registry.clear()

    assert len(registry) == 0
    assert not state1.path.exists()
    assert not state2.path.exists()


def test_clear_removes_corrupted_states(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    registry.path(OTHER_STATE_NAME).write_text("not valid json")

    registry.clear()

    assert len(registry) == 0


def test_clear_stops_at_the_first_failure(reader, registry, tmp_file, config):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / "blocked.state.json").mkdir()

    with pytest.raises(StateError, match="io failed"):
        registry.clear()

    assert (config.state_dir / "blocked.state.json").exists()


def test_registries_sharing_a_state_dir_see_each_other(reader, config, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    other = PReader(config=config)

    del other.states[TEST_STATE_NAME]

    assert TEST_STATE_NAME not in reader.states


def test_getitem_returns_the_payload_name(make_reader, tmp_file):
    reader = make_reader(verify_state=False)
    state_path = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    payload = json.loads(state_path.read_text())
    payload["name"] = "a-different-name"

    state_path.write_text(json.dumps(payload))

    assert reader.states[TEST_STATE_NAME].name == "a-different-name"


def test_getitem_raises_when_a_field_has_the_wrong_type(make_reader, tmp_file):
    reader = make_reader(verify_state=False)
    state_path = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    payload = json.loads(state_path.read_text())
    payload["position"] = "abc"

    state_path.write_text(json.dumps(payload))

    with pytest.raises(StateError, match="invalid type"):
        reader.states[TEST_STATE_NAME]


def test_len_counts_states_that_all_rejects(reader, registry, tmp_file, config):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / "broken.state.json").write_text("{ not valid json")

    assert len(registry) == 2

    with pytest.raises(StateError, match="line 1 column"):
        registry.all()


def test_search_matches_everything_with_an_empty_pattern(reader, registry, tmp_file):
    for name in ("foo-1", "bar-1"):
        reader.bytes(tmp_file, state=name).state.save()

    assert set(registry.search("")) == {"foo-1", "bar-1"}


def test_search_matches_anywhere_in_the_name(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state="myjob-2").state.save()

    assert set(registry.search("ob")) == {TEST_STATE_NAME, "myjob-2"}


def test_search_treats_the_pattern_as_a_regex(reader, registry, tmp_file):
    for name in ("foo-1", "foo.1", "fooX1"):
        reader.bytes(tmp_file, state=name).state.save()

    assert set(registry.search("foo.1")) == {"foo-1", "foo.1", "fooX1"}


@pytest.mark.parametrize("corrupted", ["foo-1", "foo-2", "foo-3"], ids=["first", "middle", "last"])
def test_all_raises_when_any_state_is_corrupt(reader, registry, tmp_file, config, corrupted):
    for name in ("foo-1", "foo-2", "foo-3"):
        reader.bytes(tmp_file, state=name).state.save()

    (config.state_dir / f"{corrupted}.state.json").write_text("{ not valid json")

    with pytest.raises(StateError, match="line 1 column"):
        registry.all()


@pytest.mark.parametrize(
    ("call", "expected"),
    [
        (lambda registry: list(registry.names()), []),
        (lambda registry: list(registry), []),
        (lambda registry: list(registry.all()), []),
        (lambda registry: list(registry.search(TEST_STATE_NAME)), []),
        (lambda registry: registry.clear(), None),
    ],
    ids=["names", "iter", "all", "search", "clear"],
)
def test_missing_state_dir_yields_empty_results(registry, config, call, expected):
    assert not config.state_dir.exists()
    assert call(registry) == expected


def test_names_creates_the_state_dir(registry, config):
    assert not config.state_dir.exists()

    list(registry.names())

    assert config.state_dir.is_dir()


@pytest.mark.parametrize(
    "call",
    [
        lambda registry: list(registry.names()),
        lambda registry: len(registry),
        lambda registry: registry.all(),
        lambda registry: registry.clear(),
        lambda registry: list(registry.search(TEST_STATE_NAME)),
    ],
    ids=["names", "len", "all", "clear", "search"],
)
def test_registry_raises_when_the_state_dir_is_a_file(make_reader, tmp_path, call):
    (tmp_path / "preader").write_text("not a directory")

    with pytest.raises(StateError, match="AlreadyExists"):
        call(make_reader().states)


def test_exists(reader, registry, tmp_file):
    assert not registry.exists(TEST_STATE_NAME)

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert registry.exists(TEST_STATE_NAME)


def test_load_returns_saved_state(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    state = registry.load(TEST_STATE_NAME)

    assert state.name == TEST_STATE_NAME
    assert state.position == 0
    assert state.file.path == tmp_file


def test_load_raises_when_missing(registry):
    with pytest.raises(KeyError):
        registry.load(TEST_STATE_NAME)


def test_delete_removes_saved_state(reader, registry, tmp_file):
    state_path = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    registry.delete(TEST_STATE_NAME)

    assert not registry.exists(TEST_STATE_NAME)
    assert not state_path.exists()


def test_delete_raises_when_missing(registry):
    with pytest.raises(KeyError):
        registry.delete(TEST_STATE_NAME)
