import json
import os

import pytest
from constants import (
    TEST_STATE_NAME,
    TEST_UNSAFE_STATE_NAME_IDS,
    TEST_UNSAFE_STATE_NAMES,
    TEST_WINDOWS_UNSAFE_STATE_NAME_IDS,
    TEST_WINDOWS_UNSAFE_STATE_NAMES,
)
from preader import Config, PReader, StateError

MISSING_STATE_NAME = "job-missing"
OTHER_STATE_NAME = "job-2"
NESTED_STATE_NAME = os.path.join("sub-1", "sub-2", "job-1")
DEEP_STATE_NAME = os.path.join("sub-1", "sub-2", "sub-3", "sub-4", "job-1")
EVERY_DEPTH = (TEST_STATE_NAME, NESTED_STATE_NAME, DEEP_STATE_NAME)


def test_len(reader, registry, tmp_file):
    assert len(registry) == 0

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert len(registry) == 1


def test_contains(reader, registry, tmp_file):
    assert TEST_STATE_NAME not in registry

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert TEST_STATE_NAME in registry


@pytest.mark.parametrize(
    "name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS
)
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


def test_iter_survives_a_midway_delete(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=OTHER_STATE_NAME).state.save()

    iterator = iter(registry)
    first = next(iterator)
    deleted = OTHER_STATE_NAME if first == TEST_STATE_NAME else TEST_STATE_NAME

    registry.delete(deleted)

    assert set(iterator) <= {deleted}
    assert set(registry) == {first}


@pytest.mark.parametrize(
    "lookup",
    [
        lambda registry: registry[TEST_STATE_NAME],
        lambda registry: registry.load(TEST_STATE_NAME),
        lambda registry: registry.find(TEST_STATE_NAME),
    ],
    ids=["getitem", "load", "find"],
)
def test_lookups_return_the_saved_state(reader, registry, tmp_file, lookup):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    state = lookup(registry)

    assert state.name == TEST_STATE_NAME
    assert state.position == 0
    assert state.file.path == tmp_file


def test_getitem_accepts_an_already_suffixed_name(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    state = registry[f"{TEST_STATE_NAME}.state.json"]

    assert state.name == TEST_STATE_NAME
    assert state.file.path == tmp_file


@pytest.mark.parametrize(
    "lookup",
    [
        lambda registry: registry[MISSING_STATE_NAME],
        lambda registry: registry.load(MISSING_STATE_NAME),
        lambda registry: registry.__delitem__(MISSING_STATE_NAME),
        lambda registry: registry.delete(MISSING_STATE_NAME),
    ],
    ids=["getitem", "load", "delitem", "delete"],
)
def test_lookups_raise_when_missing(registry, lookup):
    with pytest.raises(KeyError):
        lookup(registry)


@pytest.mark.parametrize(
    "name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_getitem_raises_when_name_is_unsafe(registry, name):
    with pytest.raises(KeyError):
        registry[name]


def test_getitem_raises_when_state_is_a_directory(registry):
    registry.path(TEST_STATE_NAME).mkdir(parents=True)

    with pytest.raises(StateError, match="state not found"):
        registry[TEST_STATE_NAME]


def test_delete_raises_when_state_is_a_directory(registry, config):
    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / f"{TEST_STATE_NAME}.state.json").mkdir()

    with pytest.raises(StateError, match="io failed"):
        del registry[TEST_STATE_NAME]


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


@pytest.mark.usefixtures("requires_non_utf8_names")
def test_names_ignores_a_non_utf8_state(reader, registry, config, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / os.fsdecode(b"ghost-\xff.state.json")).write_text("{}")

    assert list(registry.names()) == [TEST_STATE_NAME]


def test_clear_keeps_non_state_files(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    unrelated = registry.path(TEST_STATE_NAME).parent / "README.md"
    unrelated.write_text("not a state")

    registry.clear()

    assert len(registry) == 0
    assert unrelated.exists()


@pytest.mark.parametrize("state", EVERY_DEPTH, ids=["flat", "nested", "deep"])
def test_save_creates_the_state_file_under_its_name(reader, config, tmp_file, state):
    path = reader.bytes(tmp_file, state=state).state.save()

    assert path == config.state_dir / f"{state}.state.json"
    assert path.is_file()


def test_forward_slashes_resolve_to_the_native_name(reader, registry, tmp_file):
    saved = reader.bytes(tmp_file, state="sub-1/sub-2/job-1").state
    saved.save()

    assert saved.name == NESTED_STATE_NAME
    assert list(registry.names()) == [NESTED_STATE_NAME]


@pytest.mark.parametrize(
    "state",
    ["job-1/.state.json", "sub/.state.json", "job-1/.state.json/.state.json"],
    ids=["flat", "nested", "repeated"],
)
def test_save_raises_when_the_name_reduces_to_nothing(reader, tmp_file, state):
    with pytest.raises(StateError, match="path must not be empty"):
        reader.bytes(tmp_file, state=state).state.save()


def test_save_keeps_a_suffix_shaped_directory_in_the_name(
    reader, registry, config, tmp_file
):
    name = os.path.join("sub-1", ".state.json", "sub-2", "job-1")

    path = reader.bytes(tmp_file, state=f"{name}.state.json").state.save()

    assert path == config.state_dir / f"{name}.state.json"
    assert list(registry.names()) == [name]
    assert registry[name].name == name


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_symlinked_state(reader, registry, config, tmp_file):
    real = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / f"{OTHER_STATE_NAME}.state.json").symlink_to(real)

    assert list(registry.names()) == [TEST_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_broken_symlink(reader, registry, config, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / "broken.state.json").symlink_to(
        config.state_dir / "gone.state.json"
    )

    assert list(registry.names()) == [TEST_STATE_NAME]
    assert "broken" not in registry


@pytest.mark.usefixtures("requires_symlinks")
def test_names_does_not_descend_into_a_symlinked_directory(
    reader, registry, config, tmp_file
):
    outside = config.state_dir.parent / "outside"
    outside.mkdir(parents=True)

    reader = PReader(config=Config(state_dir=outside))
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    assert list(registry.names()) == []
    assert os.path.join("link", TEST_STATE_NAME) not in registry


@pytest.mark.usefixtures("requires_symlinks")
def test_save_raises_when_the_name_escapes_through_a_symlink(
    reader, config, tmp_path, tmp_file
):
    outside = tmp_path / "outside"
    outside.mkdir(parents=True)

    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    with pytest.raises(StateError, match="path escapes root"):
        reader.bytes(tmp_file, state=os.path.join("link", TEST_STATE_NAME)).state.save()

    assert not list(outside.iterdir())


@pytest.mark.usefixtures("requires_symlinks")
def test_delete_raises_when_the_name_escapes_through_a_symlink(
    registry, config, tmp_path
):
    outside = tmp_path / "outside"
    outside.mkdir(parents=True)

    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    victim = outside / f"{TEST_STATE_NAME}.state.json"
    victim.write_text("{}")

    with pytest.raises(KeyError):
        del registry[os.path.join("link", TEST_STATE_NAME)]

    assert victim.is_file()


@pytest.mark.usefixtures("requires_symlinks")
def test_getitem_raises_when_the_state_is_a_symlink(reader, registry, config, tmp_file):
    real = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / f"{OTHER_STATE_NAME}.state.json").symlink_to(real)

    with pytest.raises(KeyError):
        registry[OTHER_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_symlink_that_leaves_the_state_dir(
    reader, registry, config, tmp_path, tmp_file
):
    outside = tmp_path / "outside"
    outside.mkdir(parents=True)

    target = (
        PReader(config=Config(state_dir=outside))
        .bytes(tmp_file, state=TEST_STATE_NAME)
        .state.save()
    )

    reader.bytes(tmp_file, state=OTHER_STATE_NAME).state.save()
    (config.state_dir / "evil.state.json").symlink_to(target)

    assert list(registry.names()) == [OTHER_STATE_NAME]
    assert "evil" not in registry


def test_names_lists_states_at_every_depth(reader, registry, tmp_file):
    for state in EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).state.save()

    assert sorted(registry.names()) == sorted(EVERY_DEPTH)
    assert len(registry) == len(EVERY_DEPTH)


def test_search_matches_states_at_every_depth(reader, registry, tmp_file):
    for state in EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).state.save()

    assert sorted(registry.search(TEST_STATE_NAME)) == sorted(EVERY_DEPTH)


@pytest.mark.parametrize(
    "suffix", [".state.json", ".state.json.state.json"], ids=["once", "twice"]
)
def test_getitem_finds_a_state_saved_with_a_suffix(reader, registry, tmp_file, suffix):
    reader.bytes(tmp_file, state=f"{TEST_STATE_NAME}{suffix}").state.save()

    assert registry[f"{TEST_STATE_NAME}{suffix}"].name == TEST_STATE_NAME


def test_names_match_the_state_names(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=f"{TEST_STATE_NAME}.state.json").state.save()
    reader.bytes(tmp_file, state=f"{NESTED_STATE_NAME}.state.json").state.save()
    reader.bytes(tmp_file, state=f"{DEEP_STATE_NAME}.state.json").state.save()

    assert {name: registry[name].name for name in registry.names()} == {
        TEST_STATE_NAME: TEST_STATE_NAME,
        NESTED_STATE_NAME: NESTED_STATE_NAME,
        DEEP_STATE_NAME: DEEP_STATE_NAME,
    }


def test_clear_removes_states_at_every_depth(reader, registry, config, tmp_file):
    for state in EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).state.save()

    registry.clear()

    assert len(registry) == 0
    assert not list(config.state_dir.rglob("*.state.json"))


def test_search_returns_nothing_without_a_match(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert list(registry.search("no-such-name")) == []


def test_find_returns_none_when_missing(registry):
    assert registry.find("missing") is None


@pytest.mark.parametrize(
    "name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_find_returns_none_when_name_is_unsafe(registry, name):
    assert registry.find(name) is None


def test_find_raises_when_state_is_corrupted(registry, reader, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    registry.path(TEST_STATE_NAME).write_text("not valid json")

    with pytest.raises(StateError, match="line 1 column"):
        registry.find(TEST_STATE_NAME)


def test_search_filters_by_pattern(reader, registry, tmp_file):
    reader.bytes(tmp_file, state="foo-1").state.save()
    reader.bytes(tmp_file, state="foo-2").state.save()
    reader.bytes(tmp_file, state="bar-1").state.save()

    assert set(registry.search("^foo")) == {"foo-1", "foo-2"}


def test_delitem_removes_saved_state(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    del registry[TEST_STATE_NAME]

    assert TEST_STATE_NAME not in registry


@pytest.mark.parametrize(
    "name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_delitem_raises_when_name_is_unsafe(registry, name):
    with pytest.raises(KeyError):
        del registry[name]


def test_path_accepts_an_already_suffixed_name(registry):
    assert registry.path(f"{TEST_STATE_NAME}.state.json") == registry.path(
        TEST_STATE_NAME
    )


@pytest.mark.parametrize(
    ("name", "message"), TEST_UNSAFE_STATE_NAMES.items(), ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_path_raises_when_name_is_unsafe(registry, name, message):
    with pytest.raises(StateError, match=message):
        registry.path(name)


@pytest.mark.skipif(
    os.name != "nt", reason="Windows path syntax is only unsafe on Windows"
)
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
    assert not state1.path().exists()
    assert not state2.path().exists()


def test_clear_removes_corrupted_states(reader, registry, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    registry.path(OTHER_STATE_NAME).write_text("not valid json")

    registry.clear()

    assert len(registry) == 0


def test_names_ignores_a_state_shaped_directory(reader, registry, tmp_file, config):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / "archive.state.json").mkdir()

    assert list(registry.names()) == [TEST_STATE_NAME]
    assert len(registry) == 1
    assert [state.name for state in registry.all()] == [TEST_STATE_NAME]


def test_clear_keeps_a_state_shaped_directory(reader, registry, tmp_file, config):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    directory = config.state_dir / "archive.state.json"
    directory.mkdir()

    registry.clear()

    assert len(registry) == 0
    assert directory.is_dir()


def test_a_state_shaped_parent_does_not_hide_its_states(reader, registry, tmp_file):
    nested = reader.bytes(tmp_file, state="archive.state.json/job-1").state.save()

    assert list(registry.names()) == ["archive.state.json/job-1".replace("/", os.sep)]
    assert len(registry) == 1

    registry.clear()

    assert not nested.exists()


def test_registries_sharing_a_state_dir_see_each_other(reader, make_reader, tmp_file):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    other = make_reader()

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

    (config.state_dir / "broken.state.json").write_text("not valid json")

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


@pytest.mark.parametrize(
    "corrupted", ["foo-1", "foo-2", "foo-3"], ids=["first", "middle", "last"]
)
def test_all_raises_when_any_state_is_corrupt(
    reader, registry, tmp_file, config, corrupted
):
    for name in ("foo-1", "foo-2", "foo-3"):
        reader.bytes(tmp_file, state=name).state.save()

    (config.state_dir / f"{corrupted}.state.json").write_text("not valid json")

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
def test_registry_raises_when_the_state_dir_is_a_file(config, make_reader, call):
    config.state_dir.write_text("not a directory")

    with pytest.raises(StateError, match="AlreadyExists"):
        call(make_reader().states)


@pytest.mark.parametrize(
    ("call", "expected"),
    [
        (lambda registry: registry.exists(TEST_STATE_NAME), False),
        (lambda registry: registry.find(TEST_STATE_NAME), None),
        (lambda registry: TEST_STATE_NAME in registry, False),
    ],
    ids=["exists", "find", "contains"],
)
def test_blocked_state_dir_yields_empty_lookups(config, make_reader, call, expected):
    config.state_dir.write_text("not a directory")

    assert call(make_reader().states) == expected


@pytest.mark.parametrize(
    "call",
    [
        lambda registry: list(registry.names()),
        lambda registry: len(registry),
        lambda registry: registry.all(),
        lambda registry: registry.clear(),
    ],
    ids=["names", "len", "all", "clear"],
)
def test_registry_raises_when_a_subdirectory_is_unreadable(
    reader, registry, config, tmp_file, revoke_permissions, call
):
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    blocked = config.state_dir / "sub"
    blocked.mkdir()

    (blocked / f"{OTHER_STATE_NAME}.state.json").write_text("{}")

    revoke_permissions(blocked)

    with pytest.raises(StateError, match="io failed"):
        call(registry)


def test_exists(reader, registry, tmp_file):
    assert not registry.exists(TEST_STATE_NAME)

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert registry.exists(TEST_STATE_NAME)


def test_delete_removes_saved_state(reader, registry, tmp_file):
    state_path = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    registry.delete(TEST_STATE_NAME)

    assert not registry.exists(TEST_STATE_NAME)
    assert not state_path.exists()
