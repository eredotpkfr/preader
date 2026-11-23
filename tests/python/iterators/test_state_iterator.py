import os

from collections.abc import Callable
from pathlib import Path

import pytest

from cases.file import (
    FOREIGN_STATE_FILES,
    NON_STATE_FILES,
    UNADDRESSABLE_FILES,
    UNREPRESENTABLE_FILES,
)
from common.constants import (
    TEST_EVERY_DEPTH,
    TEST_MISSING_STATE_NAME,
    TEST_OTHER_STATE_FILE,
    TEST_OTHER_STATE_NAME,
    TEST_STATE_FILE,
    TEST_STATE_FILE_EXTENSION,
    TEST_STATE_NAME,
    TEST_SUB_STATE_NAME,
)
from preader import Config, PReader, StateRegistry


@pytest.mark.parametrize(
    "pattern", [None, TEST_STATE_NAME], ids=["without_a_pattern", "with_a_pattern"]
)
def test_state_iterator_attributes(
    registry: StateRegistry, pattern: str | None
) -> None:
    iterator = registry.search(pattern) if pattern else registry.names()

    assert iterator.state_dir == registry.state_dir
    assert iterator.pattern == pattern


@pytest.mark.parametrize(
    "pattern", [None, TEST_STATE_NAME], ids=["without_a_pattern", "with_a_pattern"]
)
def test_state_iterator_repr(
    registry: StateRegistry, expected_repr: Callable[..., str], pattern: str | None
) -> None:
    iterator = registry.search(pattern) if pattern else registry.names()

    assert repr(iterator) == expected_repr(
        "StateIterator", state_dir=registry.state_dir, pattern=pattern
    )


def test_search_returns_nothing_without_a_match(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    assert list(registry.search(TEST_MISSING_STATE_NAME)) == []


@pytest.mark.parametrize(("file", "name"), FOREIGN_STATE_FILES)
def test_clear_removes_a_foreign_state_file(
    registry: StateRegistry, make_stray: Callable[[str], Path], file: str, name: str
) -> None:
    foreign = make_stray(file)

    registry.clear()

    assert not foreign.exists()
    assert name not in registry


def test_search_matches_everything_with_an_empty_pattern(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    for name in ("foo-1", "bar-1"):
        reader.bytes(tmp_file, state=name).save()

    assert sorted(registry.search("")) == ["bar-1", "foo-1"]


def test_search_matches_states_at_every_depth(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    for state in TEST_EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).save()

    assert sorted(registry.search(TEST_STATE_NAME)) == sorted(TEST_EVERY_DEPTH)


def test_search_treats_the_pattern_as_a_regex(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    for name in ("foo-1", "foo.1", "foox1"):
        reader.bytes(tmp_file, state=name).save()

    assert sorted(registry.search("foo.1")) == ["foo-1", "foo.1", "foox1"]


@pytest.mark.usefixtures("requires_non_utf8_names")
def test_names_ignores_a_non_utf8_state(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    make_stray: Callable[[str], Path],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    make_stray(os.fsdecode(b"job-\xff.state.json"))

    assert list(registry.names()) == [TEST_STATE_NAME]


@pytest.mark.parametrize("name", NON_STATE_FILES)
def test_clear_keeps_a_non_state_file(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    make_stray: Callable[[str], Path],
    name: str,
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    unrelated = make_stray(name)

    registry.clear()

    assert len(registry) == 0
    assert unrelated.exists()


@pytest.mark.parametrize(
    ("pattern", "expected"),
    [
        ("^job-[12]$", [TEST_STATE_NAME, TEST_OTHER_STATE_NAME]),
        ("^job-1$", [TEST_STATE_NAME]),
    ],
    ids=["character_class", "anchored"],
)
def test_search_filters_the_saved_states_by_regex(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    pattern: str,
    expected: list[str],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).save()

    assert sorted(registry.search(pattern)) == expected


def test_search_matches_anywhere_in_the_name(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    reader.bytes(tmp_file, state="myjob-2").save()

    assert sorted(registry.search("ob")) == [TEST_STATE_NAME, "myjob-2"]


@pytest.mark.parametrize("name", NON_STATE_FILES)
def test_walk_ignores_a_non_state_file(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    make_stray: Callable[[str], Path],
    name: str,
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    make_stray(name)

    assert list(registry.names()) == [TEST_STATE_NAME]
    assert len(registry) == 1


def test_names_lists_every_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).save()

    assert sorted(registry.names()) == [TEST_STATE_NAME, TEST_OTHER_STATE_NAME]


def test_names_lists_states_at_every_depth(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    for state in TEST_EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).save()

    assert sorted(registry.names()) == sorted(TEST_EVERY_DEPTH)
    assert len(registry) == len(TEST_EVERY_DEPTH)


def test_clear_keeps_a_state_shaped_directory(
    reader: PReader, registry: StateRegistry, tmp_file: Path, config: Config
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    directory = config.state_dir / TEST_OTHER_STATE_FILE
    directory.mkdir()

    registry.clear()

    assert len(registry) == 0
    assert directory.is_dir()


@pytest.mark.parametrize("file", UNADDRESSABLE_FILES)
def test_clear_keeps_a_file_it_cannot_address(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    make_stray: Callable[[str], Path],
    file: str,
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    saved = registry.path(TEST_STATE_NAME)

    stray = make_stray(file)

    registry.clear()

    assert not saved.exists()
    assert stray.exists()


@pytest.mark.skipif(os.name == "nt", reason="Windows cannot create these files")
@pytest.mark.parametrize("file", UNREPRESENTABLE_FILES)
def test_clear_keeps_a_file_it_cannot_represent(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    make_stray: Callable[[str], Path],
    file: str,
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    saved = registry.path(TEST_STATE_NAME)

    stray = make_stray(file)

    registry.clear()

    assert not saved.exists()
    assert stray.exists()


def test_names_restarts_for_each_walk(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    first = list(registry.names())
    second = list(registry.names())

    assert first == [TEST_STATE_NAME]
    assert first == second


def test_search_filters_by_pattern(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state="foo-1").save()
    reader.bytes(tmp_file, state="foo-2").save()
    reader.bytes(tmp_file, state="bar-1").save()

    assert sorted(registry.search("^foo")) == ["foo-1", "foo-2"]


@pytest.mark.usefixtures("requires_non_utf8_names")
def test_clear_keeps_a_non_utf8_state(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    make_stray: Callable[[str], Path],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    saved = registry.path(TEST_STATE_NAME)

    unrepresentable = make_stray(os.fsdecode(b"job-\xff.state.json"))

    registry.clear()

    assert not saved.exists()
    assert unrepresentable.exists()


@pytest.mark.parametrize(
    "state",
    [f"{TEST_STATE_NAME}/.state.json", f"{TEST_STATE_NAME}/.state.json/.state.json"],
    ids=["flat", "repeated"],
)
def test_names_lists_a_suffix_shaped_component(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path, state: str
) -> None:
    reader.bytes(tmp_file, state=state).save()
    path = registry.path(state)

    assert path == config.state_dir / f"{state}{TEST_STATE_FILE_EXTENSION}"
    assert list(registry.names()) == [state]


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_symlinked_state(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    real = registry.path(TEST_STATE_NAME)

    alias = config.state_dir / TEST_OTHER_STATE_FILE
    alias.symlink_to(real)

    assert list(registry.names()) == [TEST_STATE_NAME]


def test_names_keeps_a_state_suffix_at_every_depth(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    names = [f"{state}{TEST_STATE_FILE_EXTENSION}" for state in TEST_EVERY_DEPTH]

    for name in names:
        reader.bytes(tmp_file, state=name).save()

    assert sorted(registry[name].name for name in registry.names()) == sorted(names)


@pytest.mark.skipif(os.name == "nt", reason="Windows cannot create these files")
@pytest.mark.parametrize("file", UNREPRESENTABLE_FILES)
def test_walk_skips_a_file_it_cannot_represent(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    make_stray: Callable[[str], Path],
    file: str,
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    make_stray(file)

    assert list(registry.names()) == [TEST_STATE_NAME]
    assert len(registry) == 1
    assert [state.name for state in registry.all()] == [TEST_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_broken_symlink(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    broken = config.state_dir / TEST_OTHER_STATE_FILE
    broken.symlink_to(
        config.state_dir / f"{TEST_MISSING_STATE_NAME}{TEST_STATE_FILE_EXTENSION}"
    )

    assert list(registry.names()) == [TEST_STATE_NAME]
    assert TEST_OTHER_STATE_NAME not in registry


def test_walk_ignores_a_state_shaped_directory(
    reader: PReader, registry: StateRegistry, tmp_file: Path, config: Config
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    (config.state_dir / TEST_OTHER_STATE_FILE).mkdir()

    assert list(registry.names()) == [TEST_STATE_NAME]
    assert len(registry) == 1
    assert [state.name for state in registry.all()] == [TEST_STATE_NAME]


def test_names_survives_deleting_the_yielded_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).save()

    iterator = registry.names()
    first = next(iterator)

    registry.delete(first)

    rest = list(iterator)

    assert sorted([first, *rest]) == [TEST_STATE_NAME, TEST_OTHER_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
def test_names_does_not_descend_into_a_symlinked_directory(
    make_reader: Callable[..., PReader],
    registry: StateRegistry,
    config: Config,
    tmp_path: Path,
    tmp_file: Path,
) -> None:
    outside = tmp_path / "outside"

    make_reader(state_dir=outside).bytes(tmp_file, state=TEST_STATE_NAME).save()

    (outside / "link.bin").symlink_to(outside / "missing.bin")

    config.state_dir.mkdir()
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    assert list(registry.names()) == []
    assert f"link/{TEST_STATE_NAME}" not in registry


def test_iter_survives_a_midway_delete(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).save()

    iterator = iter(registry)
    first = next(iterator)
    deleted = TEST_OTHER_STATE_NAME if first == TEST_STATE_NAME else TEST_STATE_NAME

    registry.delete(deleted)

    assert set(iterator) <= {deleted}
    assert set(registry) == {first}


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

    elsewhere = make_reader(state_dir=outside)
    elsewhere.bytes(tmp_file, state=TEST_STATE_NAME).save()

    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    link = config.state_dir / TEST_OTHER_STATE_FILE
    link.symlink_to(elsewhere.states.path(TEST_STATE_NAME))

    assert list(registry.names()) == [TEST_STATE_NAME]
    assert TEST_OTHER_STATE_NAME not in registry


@pytest.mark.usefixtures("requires_symlinks")
def test_clear_does_not_descend_into_a_symlinked_directory(
    reader: PReader,
    make_reader: Callable[..., PReader],
    registry: StateRegistry,
    config: Config,
    tmp_path: Path,
    tmp_file: Path,
) -> None:
    outside = tmp_path / "outside"

    make_reader(state_dir=outside).bytes(tmp_file, state=TEST_STATE_NAME).save()

    (outside / "link.bin").symlink_to(outside / "missing.bin")

    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    saved = registry.path(TEST_STATE_NAME)

    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    registry.clear()

    assert not saved.exists()
    assert (outside / TEST_STATE_FILE).exists()


@pytest.mark.usefixtures("requires_symlinks")
def test_clear_keeps_a_symlinked_state(
    reader: PReader,
    make_reader: Callable[..., PReader],
    registry: StateRegistry,
    config: Config,
    tmp_path: Path,
    tmp_file: Path,
) -> None:
    outside = tmp_path / "outside"

    make_reader(state_dir=outside).bytes(tmp_file, state=TEST_STATE_NAME).save()
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    saved = registry.path(TEST_STATE_NAME)

    target = outside / TEST_STATE_FILE
    alias = config.state_dir / TEST_OTHER_STATE_FILE

    alias.symlink_to(target)

    registry.clear()

    assert not saved.exists()
    assert target.exists()
    assert alias.is_symlink()


@pytest.mark.parametrize("file", UNADDRESSABLE_FILES)
def test_walk_skips_a_file_it_cannot_address(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    make_stray: Callable[[str], Path],
    file: str,
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    reader.bytes(tmp_file, state=TEST_SUB_STATE_NAME).save()

    make_stray(file)

    loaded = [state.name for state in registry.all()]

    assert sorted(registry.names()) == [TEST_STATE_NAME, TEST_SUB_STATE_NAME]
    assert len(registry) == 2
    assert sorted(loaded) == [TEST_STATE_NAME, TEST_SUB_STATE_NAME]
    assert sorted(registry.search("")) == [TEST_STATE_NAME, TEST_SUB_STATE_NAME]


@pytest.mark.parametrize(("file", "name"), FOREIGN_STATE_FILES)
def test_names_lists_a_foreign_state_file_under_its_own_name(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    make_stray: Callable[[str], Path],
    file: str,
    name: str,
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    saved = registry.path(TEST_STATE_NAME)

    reader.bytes(tmp_file, state=TEST_SUB_STATE_NAME).save()
    sub = registry.path(TEST_SUB_STATE_NAME)

    foreign = make_stray(file)

    expected = sorted([TEST_STATE_NAME, TEST_SUB_STATE_NAME, name])

    assert sorted(registry.names()) == expected
    assert registry.path(name) == foreign

    del registry[name]

    assert not foreign.exists()
    assert saved.exists()
    assert sub.exists()
