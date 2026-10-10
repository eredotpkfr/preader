import contextlib
import ctypes
import json
import os
import re
import sys

from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

from cases import (
    DEVICE_STATE_NAMES,
    FOREIGN_STATE_FILES,
    INVALID_STATE_NAME_ERRORS,
    INVALID_STATE_NAMES,
    NON_STATE_FILES,
    UNADDRESSABLE_FILES,
    UNPORTABLE_CHARACTERS,
    UNREPRESENTABLE_FILES,
    VALID_STATE_NAMES,
    WINDOWS_INVALID_STATE_NAMES,
)
from constants import (
    TEST_DEEP_STATE_NAME,
    TEST_EVERY_DEPTH,
    TEST_MISSING_STATE_NAME,
    TEST_NESTED_STATE_NAME,
    TEST_OTHER_STATE_NAME,
    TEST_STATE_FILE,
    TEST_STATE_FILE_EXTENSION,
    TEST_STATE_NAME,
    TEST_SUB_STATE_NAME,
)
from preader import Config, PReader, StateError, StateRegistry


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
    ids=["without_a_pattern", "with_a_pattern"],
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


def test_len_counts_the_saved_states(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    assert len(registry) == 0

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert len(registry) == 1


def test_contains_reports_only_saved_states(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    assert TEST_STATE_NAME not in registry

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert TEST_STATE_NAME in registry


@pytest.mark.parametrize("name", INVALID_STATE_NAMES)
def test_contains_excludes_an_invalid_name(registry: StateRegistry, name: str) -> None:
    assert name not in registry


def test_names_lists_every_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).state.save()

    assert set(registry.names()) == {TEST_STATE_NAME, TEST_OTHER_STATE_NAME}


def test_iter_yields_every_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).state.save()

    assert set(registry) == {TEST_STATE_NAME, TEST_OTHER_STATE_NAME}


def test_iter_survives_a_midway_delete(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).state.save()

    iterator = iter(registry)
    first = next(iterator)
    deleted = TEST_OTHER_STATE_NAME if first == TEST_STATE_NAME else TEST_STATE_NAME

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


def test_getitem_raises_when_the_name_is_suffixed(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    with pytest.raises(KeyError, match=TEST_STATE_FILE):
        registry[TEST_STATE_FILE]


@pytest.mark.parametrize(
    "lookup",
    [
        lambda registry: registry[TEST_MISSING_STATE_NAME],
        lambda registry: registry.load(TEST_MISSING_STATE_NAME),
        lambda registry: registry.__delitem__(TEST_MISSING_STATE_NAME),
        lambda registry: registry.delete(TEST_MISSING_STATE_NAME),
    ],
    ids=["getitem", "load", "delitem", "delete"],
)
def test_lookup_raises_when_the_state_is_missing(
    registry: StateRegistry, lookup: Callable[..., Any]
) -> None:
    with pytest.raises(KeyError):
        lookup(registry)


@pytest.mark.parametrize(("name", "message"), INVALID_STATE_NAME_ERRORS)
def test_getitem_raises_when_the_name_is_invalid(
    registry: StateRegistry, name: str, message: str
) -> None:
    with pytest.raises(StateError, match=message):
        registry[name]


def test_getitem_raises_when_the_state_is_a_directory(registry: StateRegistry) -> None:
    registry.path(TEST_STATE_NAME).mkdir(parents=True)

    with pytest.raises(KeyError, match=TEST_STATE_NAME):
        registry[TEST_STATE_NAME]


def test_delitem_raises_when_the_state_is_a_directory(
    registry: StateRegistry, config: Config
) -> None:
    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / TEST_STATE_FILE).mkdir()

    with pytest.raises((IsADirectoryError, PermissionError)):
        del registry[TEST_STATE_NAME]


def test_getitem_ignores_unknown_fields(
    make_reader: Callable[..., PReader], tmp_file: Path
) -> None:
    reader = make_reader(verify_state=False)
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    path = reader.states.path(TEST_STATE_NAME)
    payload = json.loads(path.read_text(encoding="utf-8"))

    payload["foo"] = 42
    path.write_text(json.dumps(payload), encoding="utf-8")

    state = reader.states[TEST_STATE_NAME]

    assert state.name == TEST_STATE_NAME
    assert state.file.path == tmp_file


@pytest.mark.parametrize("name", NON_STATE_FILES)
def test_names_ignores_a_non_state_file(
    reader: PReader, registry: StateRegistry, tmp_file: Path, name: str
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    registry.path(TEST_STATE_NAME).parent.joinpath(name).write_text(
        "not a state", encoding="utf-8"
    )

    assert set(registry.names()) == {TEST_STATE_NAME}
    assert len(registry) == 1


@pytest.mark.usefixtures("requires_non_utf8_names")
def test_names_ignores_a_non_utf8_state(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / os.fsdecode(b"ghost-\xff.state.json")).write_text(
        "{}", encoding="utf-8"
    )

    assert list(registry.names()) == [TEST_STATE_NAME]


@pytest.mark.parametrize("name", NON_STATE_FILES)
def test_clear_keeps_a_non_state_file(
    reader: PReader, registry: StateRegistry, tmp_file: Path, name: str
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    unrelated = registry.path(TEST_STATE_NAME).parent / name
    unrelated.write_text("not a state", encoding="utf-8")

    registry.clear()

    assert len(registry) == 0
    assert unrelated.exists()


@pytest.mark.parametrize("state", TEST_EVERY_DEPTH, ids=["flat", "nested", "deep"])
def test_save_creates_the_state_file_under_its_name(
    reader: PReader,
    config: Config,
    tmp_file: Path,
    state: str,
    read_state: Callable[..., Any],
) -> None:
    saved = reader.bytes(tmp_file, state=state).state
    path = saved.save()

    assert path == config.state_dir / f"{state}{TEST_STATE_FILE_EXTENSION}"
    assert path.is_file()
    assert read_state(saved)


def test_lookup_raises_when_the_name_is_a_path(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    name = Path(TEST_NESTED_STATE_NAME)

    with pytest.raises(TypeError, match="state must be None"):
        reader.bytes(tmp_file, state=name)  # type: ignore[arg-type]

    with pytest.raises(TypeError, match="not an instance of 'str'"):
        registry[name]  # type: ignore[index]


@pytest.mark.parametrize(
    "state",
    ["job-1/.state.json", "sub/.state.json", "job-1/.state.json/.state.json"],
    ids=["flat", "nested", "repeated"],
)
def test_suffix_shaped_component_round_trips_through_names(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path, state: str
) -> None:
    path = reader.bytes(tmp_file, state=state).state.save()

    assert path == config.state_dir / f"{state}{TEST_STATE_FILE_EXTENSION}"
    assert list(registry.names()) == [state]


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_symlinked_state(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    real = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (
        config.state_dir / f"{TEST_OTHER_STATE_NAME}{TEST_STATE_FILE_EXTENSION}"
    ).symlink_to(real)

    assert list(registry.names()) == [TEST_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_broken_symlink(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / "broken.state.json").symlink_to(
        config.state_dir / "missing.state.json"
    )

    assert list(registry.names()) == [TEST_STATE_NAME]
    assert "broken" not in registry


@pytest.mark.usefixtures("requires_symlinks")
def test_names_does_not_descend_into_a_symlinked_directory(
    make_reader: Callable[..., PReader],
    registry: StateRegistry,
    config: Config,
    tmp_file: Path,
) -> None:
    outside = config.state_dir.parent / "outside"
    outside.mkdir(parents=True)

    make_reader(state_dir=outside).bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    assert list(registry.names()) == []
    assert f"link/{TEST_STATE_NAME}" not in registry


@pytest.mark.usefixtures("requires_symlinks")
def test_save_raises_when_the_name_escapes_through_a_symlink(
    reader: PReader, config: Config, tmp_path: Path, tmp_file: Path
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir(parents=True)

    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    with pytest.raises(
        StateError, match="path is a symlink or an alias of another entry"
    ):
        reader.bytes(tmp_file, state=f"link/{TEST_STATE_NAME}").state.save()

    assert not list(outside.iterdir())


@pytest.mark.usefixtures("requires_symlinks")
def test_delitem_raises_when_the_name_escapes_through_a_symlink(
    registry: StateRegistry, config: Config, tmp_path: Path
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir(parents=True)

    config.state_dir.mkdir(parents=True, exist_ok=True)
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    victim = outside / TEST_STATE_FILE
    victim.write_text("{}", encoding="utf-8")

    with pytest.raises(
        StateError, match="path is a symlink or an alias of another entry"
    ):
        del registry[f"link/{TEST_STATE_NAME}"]

    assert victim.is_file()


@pytest.mark.usefixtures("requires_symlinks")
def test_getitem_raises_when_the_state_is_a_symlink(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    real = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (
        config.state_dir / f"{TEST_OTHER_STATE_NAME}{TEST_STATE_FILE_EXTENSION}"
    ).symlink_to(real)

    with pytest.raises(
        StateError, match="path is a symlink or an alias of another entry"
    ):
        registry[TEST_OTHER_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_symlink_that_leaves_the_state_dir(
    reader: PReader,
    make_reader: Callable[..., PReader],
    registry: StateRegistry,
    config: Config,
    tmp_path: Path,
    tmp_file: Path,
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir(parents=True)

    target = (
        make_reader(state_dir=outside)
        .bytes(tmp_file, state=TEST_STATE_NAME)
        .state.save()
    )

    reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).state.save()
    (config.state_dir / "evil.state.json").symlink_to(target)

    assert list(registry.names()) == [TEST_OTHER_STATE_NAME]
    assert "evil" not in registry


def test_names_lists_states_at_every_depth(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    for state in TEST_EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).state.save()

    assert sorted(registry.names()) == sorted(TEST_EVERY_DEPTH)
    assert len(registry) == len(TEST_EVERY_DEPTH)


def test_search_matches_states_at_every_depth(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    for state in TEST_EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).state.save()

    assert sorted(registry.search(TEST_STATE_NAME)) == sorted(TEST_EVERY_DEPTH)


@pytest.mark.parametrize(
    "suffix",
    [TEST_STATE_FILE_EXTENSION, TEST_STATE_FILE_EXTENSION * 2],
    ids=["once", "twice"],
)
def test_getitem_finds_a_state_saved_with_a_suffix(
    reader: PReader, registry: StateRegistry, tmp_file: Path, suffix: str
) -> None:
    reader.bytes(tmp_file, state=f"{TEST_STATE_NAME}{suffix}").state.save()

    assert registry[f"{TEST_STATE_NAME}{suffix}"].name == f"{TEST_STATE_NAME}{suffix}"


def test_names_matches_the_state_names(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    nested = f"{TEST_NESTED_STATE_NAME}{TEST_STATE_FILE_EXTENSION}"
    deep = f"{TEST_DEEP_STATE_NAME}{TEST_STATE_FILE_EXTENSION}"

    reader.bytes(tmp_file, state=TEST_STATE_FILE).state.save()
    reader.bytes(tmp_file, state=nested).state.save()
    reader.bytes(tmp_file, state=deep).state.save()

    assert {name: registry[name].name for name in registry.names()} == {
        TEST_STATE_FILE: TEST_STATE_FILE,
        nested: nested,
        deep: deep,
    }


def test_clear_removes_states_at_every_depth(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    for state in TEST_EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).state.save()

    registry.clear()

    assert len(registry) == 0
    assert not list(config.state_dir.rglob("*.state.json"))


def test_search_returns_nothing_without_a_match(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert list(registry.search("no-such-name")) == []


def test_find_returns_none_when_the_state_is_missing(registry: StateRegistry) -> None:
    assert registry.find(TEST_MISSING_STATE_NAME) is None


@pytest.mark.parametrize("name", INVALID_STATE_NAMES)
def test_find_returns_none_when_the_name_is_invalid(
    registry: StateRegistry, name: str
) -> None:
    assert registry.find(name) is None


def test_find_returns_none_when_the_state_is_corrupt(
    registry: StateRegistry, reader: PReader, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    registry.path(TEST_STATE_NAME).write_text("not valid json", encoding="utf-8")

    assert registry.exists(TEST_STATE_NAME)
    assert registry.find(TEST_STATE_NAME) is None


def test_search_filters_by_pattern(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state="foo-1").state.save()
    reader.bytes(tmp_file, state="foo-2").state.save()
    reader.bytes(tmp_file, state="bar-1").state.save()

    assert set(registry.search("^foo")) == {"foo-1", "foo-2"}


def test_delitem_removes_the_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    del registry[TEST_STATE_NAME]

    assert TEST_STATE_NAME not in registry


@pytest.mark.parametrize(("name", "message"), INVALID_STATE_NAME_ERRORS)
def test_delitem_raises_when_the_name_is_invalid(
    registry: StateRegistry, name: str, message: str
) -> None:
    with pytest.raises(StateError, match=message):
        del registry[name]


def test_path_appends_the_extension_once(
    registry: StateRegistry, config: Config
) -> None:
    assert registry.path(TEST_STATE_FILE) == (
        config.state_dir / f"{TEST_STATE_FILE}{TEST_STATE_FILE_EXTENSION}"
    )


@pytest.mark.parametrize(("name", "message"), INVALID_STATE_NAME_ERRORS)
def test_path_raises_when_the_name_is_invalid(
    registry: StateRegistry, name: str, message: str
) -> None:
    with pytest.raises(StateError, match=message):
        registry.path(name)


def test_all_returns_every_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).state.save()

    states = {(state.name, state.position, state.file.path) for state in registry.all()}

    assert states == {
        (TEST_STATE_NAME, 0, tmp_file),
        (TEST_OTHER_STATE_NAME, 0, tmp_file),
    }


def test_clear_removes_every_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    state1 = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    state2 = reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).state

    state1.save()
    state2.save()

    registry.clear()

    assert len(registry) == 0
    assert not state1.path().exists()
    assert not state2.path().exists()


def test_clear_removes_a_corrupt_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    registry.path(TEST_OTHER_STATE_NAME).write_text("not valid json", encoding="utf-8")

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


def test_state_shaped_parent_does_not_hide_its_states(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    nested = reader.bytes(tmp_file, state="archive.state.json/job-1").state.save()

    assert list(registry.names()) == ["archive.state.json/job-1"]
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


def test_getitem_raises_when_the_payload_names_another_state(
    make_reader: Callable[..., PReader], tmp_file: Path
) -> None:
    reader = make_reader(verify_state=False)
    state_path = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    payload = json.loads(state_path.read_text(encoding="utf-8"))
    payload["name"] = "a-different-name"

    state_path.write_text(json.dumps(payload), encoding="utf-8")

    with pytest.raises(StateError, match="state name mismatch"):
        reader.states[TEST_STATE_NAME]


def test_getitem_raises_when_a_field_has_the_wrong_type(
    make_reader: Callable[..., PReader], tmp_file: Path
) -> None:
    reader = make_reader(verify_state=False)
    state_path = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    payload = json.loads(state_path.read_text(encoding="utf-8"))
    payload["position"] = "abc"

    state_path.write_text(json.dumps(payload), encoding="utf-8")

    with pytest.raises(StateError, match="invalid type"):
        reader.states[TEST_STATE_NAME]


def test_len_counts_states_that_all_rejects(
    reader: PReader, registry: StateRegistry, tmp_file: Path, config: Config
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    (config.state_dir / "broken.state.json").write_text(
        "not valid json", encoding="utf-8"
    )

    assert len(registry) == 2

    with pytest.raises(StateError, match="expected ident"):
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
    for name in ("foo-1", "foo.1", "foox1"):
        reader.bytes(tmp_file, state=name).state.save()

    assert set(registry.search("foo.1")) == {"foo-1", "foo.1", "foox1"}


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

    (config.state_dir / f"{corrupted}{TEST_STATE_FILE_EXTENSION}").write_text(
        "not valid json", encoding="utf-8"
    )

    with pytest.raises(StateError, match="expected ident"):
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
def test_walk_yields_nothing_without_a_state_dir(
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
        lambda registry: list(registry),  # noqa: PLW0108
    ],
    ids=["names", "len", "all", "clear", "search", "iter"],
)
def test_walk_raises_when_the_state_dir_is_a_file(
    config: Config, make_reader: Callable[..., PReader], call: Callable[..., Any]
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    with pytest.raises(FileExistsError):
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
def test_lookups_answer_when_the_state_dir_is_a_file(
    config: Config,
    make_reader: Callable[..., PReader],
    call: Callable[..., Any],
    expected: object,
) -> None:
    config.state_dir.write_bytes(b"not a directory")

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
def test_walk_raises_when_a_subdirectory_is_unreadable(
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

    (blocked / f"{TEST_OTHER_STATE_NAME}{TEST_STATE_FILE_EXTENSION}").write_text(
        "{}", encoding="utf-8"
    )

    revoke_permissions(blocked)

    with pytest.raises(PermissionError):
        call(registry)


def test_exists_reports_only_saved_states(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    assert not registry.exists(TEST_STATE_NAME)

    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert registry.exists(TEST_STATE_NAME)


def test_delete_removes_the_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    state_path = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    registry.delete(TEST_STATE_NAME)

    assert not registry.exists(TEST_STATE_NAME)
    assert not state_path.exists()


@pytest.mark.parametrize("name", VALID_STATE_NAMES)
def test_saved_name_round_trips_through_names(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path, name: str
) -> None:
    path = reader.bytes(tmp_file, state=name).state.save()

    assert path == config.state_dir / f"{name}{TEST_STATE_FILE_EXTENSION}"
    assert json.loads(path.read_text(encoding="utf-8"))["name"] == name
    assert list(registry.names()) == [name]
    assert list(registry.search(f"^{re.escape(name)}$")) == [name]
    assert registry[name].name == name
    assert registry.path(name) == path

    del registry[name]

    assert not path.exists()

    reader.bytes(tmp_file, state=name).state.save()
    registry.clear()

    assert not path.exists()


def test_lookup_raises_when_the_name_is_not_utf8(registry: StateRegistry) -> None:
    name = "job-\udcff"

    with pytest.raises(UnicodeEncodeError):
        registry.find(name)

    with pytest.raises(UnicodeEncodeError):
        registry[name]

    with pytest.raises(UnicodeEncodeError):
        del registry[name]

    with pytest.raises(UnicodeEncodeError):
        registry.path(name)


@pytest.mark.parametrize("name", WINDOWS_INVALID_STATE_NAMES)
def test_lookup_raises_when_the_name_is_not_portable(
    registry: StateRegistry, name: str
) -> None:
    assert registry.find(name) is None
    assert name not in registry

    with pytest.raises(StateError, match="path is invalid"):
        registry[name]

    with pytest.raises(StateError, match="path is invalid"):
        del registry[name]

    with pytest.raises(StateError, match="path is invalid"):
        registry.path(name)


@pytest.mark.parametrize(
    "shape", ["job{}1", "sub{}/job-1"], ids=["inside_the_name", "inside_a_directory"]
)
@pytest.mark.parametrize("character", UNPORTABLE_CHARACTERS)
def test_lookup_raises_when_a_character_is_not_portable(
    registry: StateRegistry, character: str, shape: str
) -> None:
    name = shape.format(character)

    assert registry.find(name) is None
    assert name not in registry

    with pytest.raises(StateError, match="path is invalid"):
        registry[name]

    with pytest.raises(StateError, match="path is invalid"):
        del registry[name]

    with pytest.raises(StateError, match="path is invalid"):
        registry.path(name)


@pytest.mark.parametrize("name", DEVICE_STATE_NAMES)
def test_lookup_raises_when_the_name_is_a_device(
    registry: StateRegistry, name: str
) -> None:
    assert registry.find(name) is None
    assert name not in registry

    with pytest.raises(StateError, match="path is invalid"):
        registry[name]

    with pytest.raises(StateError, match="path is invalid"):
        del registry[name]

    with pytest.raises(StateError, match="path is invalid"):
        registry.path(name)


@pytest.mark.parametrize(
    ("name", "alias"),
    [("job-1", "JOB-1"), ("café", "cafe\u0301"), ("sub/job-1", "SUB/job-1")],
    ids=["uppercase", "decomposed", "uppercase_directory"],
)
def test_alias_never_touches_the_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path, name: str, alias: str
) -> None:
    saved = reader.bytes(tmp_file, state=name).state.save()
    before = saved.read_bytes()

    with contextlib.suppress(KeyError, StateError):
        del registry[alias]

    with contextlib.suppress(StateError, OSError):
        reader.bytes(tmp_file, state=alias).state.save()

    assert name in registry.names()
    assert saved.read_bytes() == before


@pytest.mark.parametrize(("file", "name"), FOREIGN_STATE_FILES)
def test_foreign_state_file_is_listed_under_its_own_name(
    reader: PReader,
    registry: StateRegistry,
    config: Config,
    tmp_file: Path,
    file: str,
    name: str,
) -> None:
    saved = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    sub = reader.bytes(tmp_file, state=TEST_SUB_STATE_NAME).state.save()

    foreign = config.state_dir / file
    foreign.parent.mkdir(parents=True, exist_ok=True)
    foreign.write_text("{}", encoding="utf-8")

    assert sorted(registry.names()) == sorted(
        [TEST_STATE_NAME, TEST_SUB_STATE_NAME, name]
    )
    assert registry.path(name) == foreign

    del registry[name]

    assert not foreign.exists()
    assert saved.exists()
    assert sub.exists()


@pytest.mark.parametrize("file", UNADDRESSABLE_FILES)
def test_walk_skips_a_file_it_cannot_address(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path, file: str
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()
    reader.bytes(tmp_file, state=TEST_SUB_STATE_NAME).state.save()

    stray = config.state_dir / file
    stray.parent.mkdir(parents=True, exist_ok=True)
    stray.write_text("{}", encoding="utf-8")

    assert sorted(registry.names()) == [TEST_STATE_NAME, TEST_SUB_STATE_NAME]
    assert len(registry) == 2
    assert sorted(state.name for state in registry.all()) == [
        TEST_STATE_NAME,
        TEST_SUB_STATE_NAME,
    ]


@pytest.mark.parametrize("file", UNADDRESSABLE_FILES)
def test_clear_keeps_a_file_it_cannot_address(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path, file: str
) -> None:
    saved = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    stray = config.state_dir / file
    stray.parent.mkdir(parents=True, exist_ok=True)
    stray.write_text("{}", encoding="utf-8")

    registry.clear()

    assert not saved.exists()
    assert stray.exists()


def test_path_uses_the_native_separator(
    registry: StateRegistry, config: Config
) -> None:
    path = registry.path(TEST_NESTED_STATE_NAME)

    assert str(path) == str(config.state_dir / "sub-1" / "sub-2" / TEST_STATE_FILE)


def test_delitem_removes_only_the_hard_link(
    registry: StateRegistry, config: Config, tmp_path: Path
) -> None:
    outside = tmp_path / "outside.txt"
    outside.write_text("foo", encoding="utf-8")

    link = config.state_dir / TEST_STATE_FILE
    link.parent.mkdir(parents=True, exist_ok=True)
    link.hardlink_to(outside)

    del registry[TEST_STATE_NAME]

    assert not link.exists()
    assert outside.read_text(encoding="utf-8") == "foo"


@pytest.mark.usefixtures("requires_symlinks")
def test_symlinked_state_dir_is_used_as_the_root(
    make_reader: Callable[..., PReader], tmp_file: Path, tmp_path: Path
) -> None:
    real = tmp_path / "real"
    real.mkdir()

    linked = tmp_path / "linked"
    linked.symlink_to(real, target_is_directory=True)

    reader = make_reader(state_dir=linked)
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    assert (real / TEST_STATE_FILE).is_file()
    assert list(reader.states.names()) == [TEST_STATE_NAME]
    assert reader.states[TEST_STATE_NAME].name == TEST_STATE_NAME

    del reader.states[TEST_STATE_NAME]

    assert list(real.iterdir()) == []


def test_delitem_raises_when_the_name_is_suffixed(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    saved = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    with pytest.raises(KeyError):
        del registry[TEST_STATE_FILE]

    assert saved.exists()


@pytest.mark.usefixtures("requires_symlinks")
def test_lookup_raises_when_the_name_is_a_directory_alias(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    saved = reader.bytes(tmp_file, state="real/job-1").state.save()
    before = saved.read_bytes()

    (config.state_dir / "alias").symlink_to(saved.parent, target_is_directory=True)

    assert registry.find("alias/job-1") is None
    assert "alias/job-1" not in registry
    assert list(registry.names()) == ["real/job-1"]

    with pytest.raises(
        StateError, match="path is a symlink or an alias of another entry"
    ):
        registry["alias/job-1"]

    with pytest.raises(
        StateError, match="path is a symlink or an alias of another entry"
    ):
        del registry["alias/job-1"]

    with pytest.raises(
        StateError, match="path is a symlink or an alias of another entry"
    ):
        reader.bytes(tmp_file, state="alias/job-1")

    assert saved.read_bytes() == before


@pytest.mark.skipif(os.name == "nt", reason="Windows cannot create these files")
@pytest.mark.parametrize("file", UNREPRESENTABLE_FILES)
def test_walk_skips_a_file_it_cannot_represent(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path, file: str
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    stray = config.state_dir / file
    stray.parent.mkdir(parents=True, exist_ok=True)
    stray.write_text("{}", encoding="utf-8")

    assert list(registry.names()) == [TEST_STATE_NAME]
    assert len(registry) == 1


@pytest.mark.skipif(os.name == "nt", reason="Windows cannot create these files")
@pytest.mark.parametrize("file", UNREPRESENTABLE_FILES)
def test_clear_keeps_a_file_it_cannot_represent(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path, file: str
) -> None:
    saved = reader.bytes(tmp_file, state=TEST_STATE_NAME).state.save()

    stray = config.state_dir / file
    stray.parent.mkdir(parents=True, exist_ok=True)
    stray.write_text("{}", encoding="utf-8")

    registry.clear()

    assert not saved.exists()
    assert stray.exists()


def test_lookup_raises_when_the_name_is_a_short_name_alias(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    if sys.platform != "win32":
        pytest.skip("8.3 short names exist only on Windows")

    saved = reader.bytes(tmp_file, state="longdirectoryname/job-1").state.save()
    before = saved.read_bytes()
    buffer = ctypes.create_unicode_buffer(32768)

    ctypes.windll.kernel32.GetShortPathNameW(str(saved.parent), buffer, len(buffer))

    short = Path(buffer.value).name

    if short.lower() == saved.parent.name.lower():
        pytest.skip("8.3 short names are disabled on this volume")

    with pytest.raises(
        StateError, match="path is a symlink or an alias of another entry"
    ):
        registry[f"{short}/job-1"]

    with pytest.raises(
        StateError, match="path is a symlink or an alias of another entry"
    ):
        reader.bytes(tmp_file, state=f"{short}/job-1").state.save()

    assert list(registry.names()) == ["longdirectoryname/job-1"]
    assert saved.read_bytes() == before


@pytest.mark.parametrize(
    "name",
    [f"job{chr(0xD800)}", f"job{chr(0xDFFF)}"],
    ids=["high_surrogate", "low_surrogate"],
)
def test_lookup_raises_when_the_name_has_a_lone_surrogate(
    reader: PReader, registry: StateRegistry, tmp_file: Path, name: str
) -> None:
    with pytest.raises(UnicodeEncodeError, match="surrogates not allowed"):
        registry[name]

    with pytest.raises(UnicodeEncodeError, match="surrogates not allowed"):
        registry.path(name)

    with pytest.raises(TypeError, match="state must be None"):
        reader.bytes(tmp_file, state=name)
