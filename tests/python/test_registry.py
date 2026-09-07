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
from preader import Config, PReader, State, StateError, StateRegistry

MISSING_STATE_NAME = "job-missing"
OTHER_STATE_NAME = "job-2"
NESTED_STATE_NAME = str(Path("sub-1", "sub-2", "job-1"))
DEEP_STATE_NAME = str(Path("sub-1", "sub-2", "sub-3", "sub-4", "job-1"))
EVERY_DEPTH = (TEST_STATE_NAME, NESTED_STATE_NAME, DEEP_STATE_NAME)


def test_state_dir_matches_the_config(config: Config, registry: StateRegistry) -> None:
    assert registry.state_dir == config.state_dir


def test_state_registry_repr(
    registry: StateRegistry, expected_repr: Callable[..., str]
) -> None:
    assert repr(registry) == expected_repr(
        "StateRegistry", state_dir=f"'{registry.state_dir}'"
    )


@pytest.mark.parametrize(
    ("pattern", "expected_pattern"),
    [(None, "None"), (TEST_STATE_NAME, f"'{TEST_STATE_NAME}'")],
    ids=["no_pattern", "with_pattern"],
)
def test_state_iterator_repr(
    registry: StateRegistry,
    expected_repr: Callable[..., str],
    pattern: str | None,
    expected_pattern: str,
) -> None:
    iterator = registry.search(pattern) if pattern else registry.names()

    assert repr(iterator) == expected_repr(
        "StateIterator", state_dir=f"'{registry.state_dir}'", pattern=expected_pattern
    )


def test_len(reader: PReader, registry: StateRegistry, tmp_file: Path) -> None:
    assert len(registry) == 0

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert len(registry) == 1


def test_contains(reader: PReader, registry: StateRegistry, tmp_file: Path) -> None:
    assert TEST_STATE_NAME not in registry

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert TEST_STATE_NAME in registry


@pytest.mark.parametrize(
    "name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_contains_returns_false_when_name_is_unsafe(
    registry: StateRegistry, name: str
) -> None:
    assert name not in registry


def test_names(reader: PReader, registry: StateRegistry, tmp_file: Path) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=OTHER_STATE_NAME).state.save()

    assert set(registry.names()) == {TEST_STATE_NAME, OTHER_STATE_NAME}


def test_iter(reader: PReader, registry: StateRegistry, tmp_file: Path) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=OTHER_STATE_NAME).state.save()

    assert set(registry) == {TEST_STATE_NAME, OTHER_STATE_NAME}


def test_iter_survives_a_midway_delete(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
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
def test_lookup_returns_the_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path, lookup: Callable[..., Any]
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    state = lookup(registry)

    assert state.name == TEST_STATE_NAME
    assert state.position == 0
    assert state.file.path == tmp_file


def test_getitem_accepts_an_already_suffixed_name(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
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
def test_lookup_raises_when_missing(
    registry: StateRegistry, lookup: Callable[..., Any]
) -> None:
    with pytest.raises(KeyError):
        lookup(registry)


@pytest.mark.parametrize(
    "name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_getitem_raises_when_name_is_unsafe(registry: StateRegistry, name: str) -> None:
    with pytest.raises(KeyError):
        registry[name]


def test_getitem_raises_when_state_is_a_directory(registry: StateRegistry) -> None:
    registry.path(TEST_STATE_NAME).mkdir(parents=True)

    with pytest.raises(StateError, match="state not found"):
        registry[TEST_STATE_NAME]


def test_delete_raises_when_state_is_a_directory(
    registry: StateRegistry, config: Config
) -> None:
    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / f"{TEST_STATE_NAME}.state.json").mkdir()

    with pytest.raises(StateError, match="io failed"):
        del registry[TEST_STATE_NAME]


def test_getitem_ignores_unknown_fields(
    make_reader: Callable[..., PReader], tmp_file: Path
) -> None:
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
def test_names_ignores_non_state_files(
    reader: PReader, registry: StateRegistry, tmp_file: Path, name: str
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    registry.path(TEST_STATE_NAME).parent.joinpath(name).write_text("not a state")

    assert set(registry.names()) == {TEST_STATE_NAME}
    assert len(registry) == 1


@pytest.mark.usefixtures("requires_non_utf8_names")
def test_names_ignores_a_non_utf8_state(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / os.fsdecode(b"ghost-\xff.state.json")).write_text("{}")

    assert list(registry.names()) == [TEST_STATE_NAME]


def test_clear_keeps_non_state_files(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    unrelated = registry.path(TEST_STATE_NAME).parent / "README.md"
    unrelated.write_text("not a state")

    registry.clear()

    assert len(registry) == 0
    assert unrelated.exists()


@pytest.mark.parametrize("state", EVERY_DEPTH, ids=["flat", "nested", "deep"])
def test_save_creates_the_state_file_under_its_name(
    reader: PReader,
    config: Config,
    tmp_file: Path,
    state: State,
    read_state: Callable[..., Any],
) -> None:
    saved = reader.bytes(tmp_file, state=state).state
    path = saved.save()

    assert path == config.state_dir / f"{state}.state.json"
    assert path.is_file()
    assert read_state(saved)


def test_forward_slashes_resolve_to_the_native_name(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    saved = reader.bytes(tmp_file, state="sub-1/sub-2/job-1").state
    saved.save()

    assert saved.name == NESTED_STATE_NAME
    assert list(registry.names()) == [NESTED_STATE_NAME]


@pytest.mark.parametrize(
    "state",
    ["job-1/.state.json", "sub/.state.json", "job-1/.state.json/.state.json"],
    ids=["flat", "nested", "repeated"],
)
def test_save_raises_when_the_name_reduces_to_nothing(
    reader: PReader, tmp_file: Path, state: str
) -> None:
    with pytest.raises(StateError, match="path must not be empty"):
        reader.bytes(tmp_file, state=state).state.save()


def test_save_keeps_a_suffix_shaped_directory_in_the_name(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    name = str(Path("sub-1", ".state.json", "sub-2", "job-1"))

    path = reader.bytes(tmp_file, state=f"{name}.state.json").state.save()

    assert path == config.state_dir / f"{name}.state.json"
    assert list(registry.names()) == [name]
    assert registry[name].name == name


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_symlinked_state(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    real = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / f"{OTHER_STATE_NAME}.state.json").symlink_to(real)

    assert list(registry.names()) == [TEST_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_broken_symlink(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / "broken.state.json").symlink_to(
        config.state_dir / "gone.state.json"
    )

    assert list(registry.names()) == [TEST_STATE_NAME]
    assert "broken" not in registry


@pytest.mark.usefixtures("requires_symlinks")
def test_names_does_not_descend_into_a_symlinked_directory(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    outside = config.state_dir.parent / "outside"
    outside.mkdir(parents=True)

    reader = PReader(config=Config(state_dir=outside))
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    assert list(registry.names()) == []
    assert str(Path("link", TEST_STATE_NAME)) not in registry


@pytest.mark.usefixtures("requires_symlinks")
def test_save_raises_when_the_name_escapes_through_a_symlink(
    reader: PReader, config: Config, tmp_path: Path, tmp_file: Path
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir(parents=True)

    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    with pytest.raises(StateError, match="path escapes root"):
        reader.bytes(tmp_file, state=str(Path("link", TEST_STATE_NAME))).state.save()

    assert not list(outside.iterdir())


@pytest.mark.usefixtures("requires_symlinks")
def test_delete_raises_when_the_name_escapes_through_a_symlink(
    registry: StateRegistry, config: Config, tmp_path: Path
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir(parents=True)

    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    victim = outside / f"{TEST_STATE_NAME}.state.json"
    victim.write_text("{}")

    with pytest.raises(KeyError):
        del registry[str(Path("link", TEST_STATE_NAME))]

    assert victim.is_file()


@pytest.mark.usefixtures("requires_symlinks")
def test_getitem_raises_when_the_state_is_a_symlink(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    real = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / f"{OTHER_STATE_NAME}.state.json").symlink_to(real)

    with pytest.raises(KeyError):
        registry[OTHER_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_symlink_that_leaves_the_state_dir(
    reader: PReader,
    registry: StateRegistry,
    config: Config,
    tmp_path: Path,
    tmp_file: Path,
) -> None:
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


def test_names_lists_states_at_every_depth(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    for state in EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).state.save()

    assert sorted(registry.names()) == sorted(EVERY_DEPTH)
    assert len(registry) == len(EVERY_DEPTH)


def test_search_matches_states_at_every_depth(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    for state in EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).state.save()

    assert sorted(registry.search(TEST_STATE_NAME)) == sorted(EVERY_DEPTH)


@pytest.mark.parametrize(
    "suffix", [".state.json", ".state.json.state.json"], ids=["once", "twice"]
)
def test_getitem_finds_a_state_saved_with_a_suffix(
    reader: PReader, registry: StateRegistry, tmp_file: Path, suffix: str
) -> None:
    reader.bytes(tmp_file, state=f"{TEST_STATE_NAME}{suffix}").state.save()

    assert registry[f"{TEST_STATE_NAME}{suffix}"].name == TEST_STATE_NAME


def test_names_match_the_state_names(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=f"{TEST_STATE_NAME}.state.json").state.save()
    reader.bytes(tmp_file, state=f"{NESTED_STATE_NAME}.state.json").state.save()
    reader.bytes(tmp_file, state=f"{DEEP_STATE_NAME}.state.json").state.save()

    assert {name: registry[name].name for name in registry.names()} == {
        TEST_STATE_NAME: TEST_STATE_NAME,
        NESTED_STATE_NAME: NESTED_STATE_NAME,
        DEEP_STATE_NAME: DEEP_STATE_NAME,
    }


def test_clear_removes_states_at_every_depth(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    for state in EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).state.save()

    registry.clear()

    assert len(registry) == 0
    assert not list(config.state_dir.rglob("*.state.json"))


def test_search_returns_nothing_without_a_match(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert list(registry.search("no-such-name")) == []


def test_find_returns_none_when_missing(registry: StateRegistry) -> None:
    assert registry.find("missing") is None


@pytest.mark.parametrize(
    "name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_find_returns_none_when_name_is_unsafe(
    registry: StateRegistry, name: str
) -> None:
    assert registry.find(name) is None


def test_find_raises_when_state_is_corrupted(
    registry: StateRegistry, reader: PReader, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    registry.path(TEST_STATE_NAME).write_text("not valid json")

    with pytest.raises(StateError, match="line 1 column"):
        registry.find(TEST_STATE_NAME)


def test_search_filters_by_pattern(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state="foo-1").state.save()
    reader.bytes(tmp_file, state="foo-2").state.save()
    reader.bytes(tmp_file, state="bar-1").state.save()

    assert set(registry.search("^foo")) == {"foo-1", "foo-2"}


def test_delitem_removes_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    del registry[TEST_STATE_NAME]

    assert TEST_STATE_NAME not in registry


@pytest.mark.parametrize(
    "name", TEST_UNSAFE_STATE_NAMES, ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_delitem_raises_when_name_is_unsafe(registry: StateRegistry, name: str) -> None:
    with pytest.raises(KeyError):
        del registry[name]


def test_path_accepts_an_already_suffixed_name(registry: StateRegistry) -> None:
    assert registry.path(f"{TEST_STATE_NAME}.state.json") == registry.path(
        TEST_STATE_NAME
    )


@pytest.mark.parametrize(
    ("name", "message"), TEST_UNSAFE_STATE_NAMES.items(), ids=TEST_UNSAFE_STATE_NAME_IDS
)
def test_path_raises_when_name_is_unsafe(
    registry: StateRegistry, name: str, message: str
) -> None:
    with pytest.raises(StateError, match=message):
        registry.path(name)


@pytest.mark.skipif(
    os.name != "nt", reason="Windows path syntax is only unsafe on Windows"
)
@pytest.mark.parametrize(
    "name", TEST_WINDOWS_UNSAFE_STATE_NAMES, ids=TEST_WINDOWS_UNSAFE_STATE_NAME_IDS
)
def test_path_raises_when_a_windows_name_is_unsafe(
    registry: StateRegistry, name: str
) -> None:
    with pytest.raises(StateError, match="path escapes root"):
        registry.path(name)


def test_all_returns_every_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=OTHER_STATE_NAME).state.save()

    states = {(state.name, state.position, state.file.path) for state in registry.all()}

    assert states == {(TEST_STATE_NAME, 0, tmp_file), (OTHER_STATE_NAME, 0, tmp_file)}


def test_clear_removes_saved_states(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    state1 = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    state2 = reader.bytes(tmp_file, state=OTHER_STATE_NAME).state

    state1.save()
    state2.save()

    registry.clear()

    assert len(registry) == 0
    assert not state1.path().exists()
    assert not state2.path().exists()


def test_clear_removes_corrupted_states(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    registry.path(OTHER_STATE_NAME).write_text("not valid json")

    registry.clear()

    assert len(registry) == 0


def test_names_ignores_a_state_shaped_directory(
    reader: PReader, registry: StateRegistry, tmp_file: Path, config: Config
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / "archive.state.json").mkdir()

    assert list(registry.names()) == [TEST_STATE_NAME]
    assert len(registry) == 1
    assert [state.name for state in registry.all()] == [TEST_STATE_NAME]


def test_clear_keeps_a_state_shaped_directory(
    reader: PReader, registry: StateRegistry, tmp_file: Path, config: Config
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    directory = config.state_dir / "archive.state.json"
    directory.mkdir()

    registry.clear()

    assert len(registry) == 0
    assert directory.is_dir()


def test_a_state_shaped_parent_does_not_hide_its_states(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    nested = reader.bytes(tmp_file, state="archive.state.json/job-1").state.save()

    assert list(registry.names()) == ["archive.state.json/job-1".replace("/", os.sep)]
    assert len(registry) == 1

    registry.clear()

    assert not nested.exists()


def test_registries_sharing_a_state_dir_see_each_other(
    reader: PReader, make_reader: Callable[..., PReader], tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    other = make_reader()

    del other.states[TEST_STATE_NAME]

    assert TEST_STATE_NAME not in reader.states


def test_getitem_returns_the_payload_name(
    make_reader: Callable[..., PReader], tmp_file: Path
) -> None:
    reader = make_reader(verify_state=False)
    state_path = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    payload = json.loads(state_path.read_text())
    payload["name"] = "a-different-name"

    state_path.write_text(json.dumps(payload))

    assert reader.states[TEST_STATE_NAME].name == "a-different-name"


def test_getitem_raises_when_a_field_has_the_wrong_type(
    make_reader: Callable[..., PReader], tmp_file: Path
) -> None:
    reader = make_reader(verify_state=False)
    state_path = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    payload = json.loads(state_path.read_text())
    payload["position"] = "abc"

    state_path.write_text(json.dumps(payload))

    with pytest.raises(StateError, match="invalid type"):
        reader.states[TEST_STATE_NAME]


def test_len_counts_states_that_all_rejects(
    reader: PReader, registry: StateRegistry, tmp_file: Path, config: Config
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / "broken.state.json").write_text("not valid json")

    assert len(registry) == 2

    with pytest.raises(StateError, match="line 1 column"):
        registry.all()


def test_search_matches_everything_with_an_empty_pattern(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    for name in ("foo-1", "bar-1"):
        reader.bytes(tmp_file, state=name).state.save()

    assert set(registry.search("")) == {"foo-1", "bar-1"}


def test_search_matches_anywhere_in_the_name(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state="myjob-2").state.save()

    assert set(registry.search("ob")) == {TEST_STATE_NAME, "myjob-2"}


def test_search_treats_the_pattern_as_a_regex(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    for name in ("foo-1", "foo.1", "fooX1"):
        reader.bytes(tmp_file, state=name).state.save()

    assert set(registry.search("foo.1")) == {"foo-1", "foo.1", "fooX1"}


@pytest.mark.parametrize(
    "corrupted", ["foo-1", "foo-2", "foo-3"], ids=["first", "middle", "last"]
)
def test_all_raises_when_any_state_is_corrupt(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    config: Config,
    corrupted: str,
) -> None:
    for name in ("foo-1", "foo-2", "foo-3"):
        reader.bytes(tmp_file, state=name).state.save()

    (config.state_dir / f"{corrupted}.state.json").write_text("not valid json")

    with pytest.raises(StateError, match="line 1 column"):
        registry.all()


@pytest.mark.parametrize(
    ("call", "expected"),
    [
        (lambda registry: list(registry.names()), []),
        (lambda registry: list(registry), []),  # noqa: PLW0108
        (lambda registry: list(registry.all()), []),
        (lambda registry: list(registry.search(TEST_STATE_NAME)), []),
        (lambda registry: registry.clear(), None),
    ],
    ids=["names", "iter", "all", "search", "clear"],
)
def test_missing_state_dir_yields_empty_results(
    registry: StateRegistry, config: Config, call: Callable[..., Any], expected: object
) -> None:
    assert not config.state_dir.exists()
    assert call(registry) == expected


def test_names_creates_the_state_dir(registry: StateRegistry, config: Config) -> None:
    assert not config.state_dir.exists()

    list(registry.names())

    assert config.state_dir.is_dir()


@pytest.mark.parametrize(
    "call",
    [
        lambda registry: list(registry.names()),
        lambda registry: len(registry),  # noqa: PLW0108
        lambda registry: registry.all(),
        lambda registry: registry.clear(),
        lambda registry: list(registry.search(TEST_STATE_NAME)),
    ],
    ids=["names", "len", "all", "clear", "search"],
)
def test_registry_raises_when_the_state_dir_is_a_file(
    config: Config, make_reader: Callable[..., PReader], call: Callable[..., Any]
) -> None:
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
def test_blocked_state_dir_yields_empty_lookups(
    config: Config,
    make_reader: Callable[..., PReader],
    call: Callable[..., Any],
    expected: object,
) -> None:
    config.state_dir.write_text("not a directory")

    assert call(make_reader().states) == expected


@pytest.mark.parametrize(
    "call",
    [
        lambda registry: list(registry.names()),
        lambda registry: len(registry),  # noqa: PLW0108
        lambda registry: registry.all(),
        lambda registry: registry.clear(),
    ],
    ids=["names", "len", "all", "clear"],
)
def test_registry_raises_when_a_subdirectory_is_unreadable(
    reader: PReader,
    registry: StateRegistry,
    config: Config,
    tmp_file: Path,
    revoke_permissions: Callable[[Path], Path],
    call: Callable[..., Any],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    blocked = config.state_dir / "sub"
    blocked.mkdir()

    (blocked / f"{OTHER_STATE_NAME}.state.json").write_text("{}")

    revoke_permissions(blocked)

    with pytest.raises(StateError, match="io failed"):
        call(registry)


def test_exists(reader: PReader, registry: StateRegistry, tmp_file: Path) -> None:
    assert not registry.exists(TEST_STATE_NAME)

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert registry.exists(TEST_STATE_NAME)


def test_delete_removes_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    state_path = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    registry.delete(TEST_STATE_NAME)

    assert not registry.exists(TEST_STATE_NAME)
    assert not state_path.exists()
