import contextlib
import ctypes
import json
import operator
import re
import sys

from collections.abc import Callable
from contextlib import AbstractContextManager
from pathlib import Path
from typing import Any

import pytest

from cases.name import (
    ALIAS_STATE_NAMES,
    INVALID_STATE_NAME_ERRORS,
    INVALID_STATE_NAMES,
    LONE_SURROGATE_STATE_NAMES,
    VALID_STATE_NAMES,
)
from cases.payload import MALFORMED_PAYLOADS
from common.constants import (
    TEST_EVERY_DEPTH,
    TEST_MISSING_STATE_NAME,
    TEST_NESTED_STATE_NAME,
    TEST_OTHER_STATE_FILE,
    TEST_OTHER_STATE_NAME,
    TEST_STATE_FILE,
    TEST_STATE_FILE_EXTENSION,
    TEST_STATE_NAME,
    TEST_SUB_STATE_NAME,
)
from preader import Config, PReader, StateCorruptError, StateError, StateRegistry

LOOKUPS = [
    pytest.param(lambda registry, name: registry[name], id="getitem"),
    pytest.param(lambda registry, name: registry.load(name), id="load"),
    pytest.param(
        lambda registry, name: operator.delitem(registry, name),  # noqa: PLW0108
        id="delitem",
    ),
    pytest.param(lambda registry, name: registry.delete(name), id="delete"),
    pytest.param(lambda registry, name: registry.path(name), id="path"),
]
KEYED_LOOKUPS = [lookup for lookup in LOOKUPS if lookup.id != "path"]
PROBES = [
    pytest.param(lambda registry, name: registry.find(name), id="find"),
    pytest.param(lambda registry, name: registry.exists(name), id="exists"),
    pytest.param(lambda registry, name: name in registry, id="contains"),
]
WALKS = [
    pytest.param(lambda registry: list(registry.names()), id="names"),
    pytest.param(lambda registry: len(registry), id="len"),  # noqa: PLW0108
    pytest.param(lambda registry: list(registry), id="iter"),  # noqa: PLW0108
    pytest.param(lambda registry: registry.all(), id="all"),
    pytest.param(lambda registry: list(registry.search(TEST_STATE_NAME)), id="search"),
    pytest.param(lambda registry: registry.clear(), id="clear"),
]


def test_state_registry_fields(config: Config, registry: StateRegistry) -> None:
    assert registry.state_dir == config.state_dir


def test_state_registry_repr(
    registry: StateRegistry, expected_repr: Callable[..., str]
) -> None:
    assert repr(registry) == expected_repr(
        "StateRegistry", state_dir=registry.state_dir
    )


def test_path_joins_the_state_dir(registry: StateRegistry, config: Config) -> None:
    assert registry.path(TEST_STATE_NAME) == config.state_dir / TEST_STATE_FILE


@pytest.mark.parametrize("lookup", PROBES)
def test_lookup_answers_when_the_state_dir_is_a_file(
    config: Config, registry: StateRegistry, lookup: Callable[..., Any]
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    assert not lookup(registry, TEST_STATE_NAME)


@pytest.mark.parametrize("lookup", LOOKUPS + PROBES)
@pytest.mark.parametrize("name", LONE_SURROGATE_STATE_NAMES)
def test_lookup_raises_when_the_name_has_a_lone_surrogate(
    registry: StateRegistry, lookup: Callable[..., Any], name: str
) -> None:
    with pytest.raises(UnicodeEncodeError, match="surrogates not allowed"):
        lookup(registry, name)


@pytest.mark.parametrize("lookup", KEYED_LOOKUPS)
def test_lookup_raises_when_the_state_is_missing(
    registry: StateRegistry, lookup: Callable[..., Any]
) -> None:
    with pytest.raises(KeyError, match=TEST_MISSING_STATE_NAME):
        lookup(registry, TEST_MISSING_STATE_NAME)


@pytest.mark.parametrize("lookup", LOOKUPS)
@pytest.mark.parametrize(("name", "message"), INVALID_STATE_NAME_ERRORS)
def test_lookup_raises_when_the_name_is_invalid(
    registry: StateRegistry, lookup: Callable[..., Any], name: str, message: str
) -> None:
    with pytest.raises(ValueError, match=message):
        lookup(registry, name)


@pytest.mark.parametrize(
    ("call", "expected"),
    [
        (lambda registry: list(registry.names()), []),
        (lambda registry: len(registry), 0),  # noqa: PLW0108
        (lambda registry: list(registry), []),  # noqa: PLW0108
        (lambda registry: list(registry.all()), []),
        (lambda registry: list(registry.search(TEST_STATE_NAME)), []),
        (lambda registry: registry.clear(), None),
    ],
    ids=["names", "len", "iter", "all", "search", "clear"],
)
def test_walk_yields_nothing_when_the_state_dir_is_missing(
    registry: StateRegistry, config: Config, call: Callable[..., Any], expected: object
) -> None:
    assert call(registry) == expected
    assert not config.state_dir.exists()


@pytest.mark.parametrize("name", LONE_SURROGATE_STATE_NAMES)
def test_resync_raises_when_the_name_has_a_lone_surrogate(
    registry: StateRegistry, tmp_file: Path, name: str
) -> None:
    with pytest.raises(UnicodeEncodeError, match="surrogates not allowed"):
        registry.resync(name, tmp_file)


@pytest.mark.parametrize("lookup", LOOKUPS)
@pytest.mark.parametrize("name", VALID_STATE_NAMES)
def test_lookup_accepts_a_valid_name(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    lookup: Callable[..., Any],
    name: str,
) -> None:
    reader.bytes(tmp_file, state=name).save()

    lookup(registry, name)


@pytest.mark.parametrize("lookup", PROBES)
@pytest.mark.parametrize("name", VALID_STATE_NAMES)
def test_lookup_finds_a_state_saved_under_a_valid_name(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    lookup: Callable[..., Any],
    name: str,
) -> None:
    reader.bytes(tmp_file, state=name).save()

    assert lookup(registry, name)


@pytest.mark.parametrize(("name", "message"), INVALID_STATE_NAME_ERRORS)
def test_resync_raises_when_the_name_is_invalid(
    registry: StateRegistry, tmp_file: Path, name: str, message: str
) -> None:
    with pytest.raises(ValueError, match=message):
        registry.resync(name, tmp_file)


@pytest.mark.parametrize("lookup", LOOKUPS + PROBES)
def test_lookup_raises_when_the_name_is_a_path(
    registry: StateRegistry, lookup: Callable[..., Any]
) -> None:
    with pytest.raises(TypeError, match="not an instance of 'str'"):
        lookup(registry, Path(TEST_NESTED_STATE_NAME))


@pytest.mark.parametrize("lookup", PROBES)
def test_lookup_answers_when_the_state_is_a_directory(
    registry: StateRegistry, lookup: Callable[..., Any]
) -> None:
    registry.path(TEST_STATE_NAME).mkdir(parents=True)

    assert not lookup(registry, TEST_STATE_NAME)


@pytest.mark.parametrize("lookup", PROBES)
def test_lookup_answers_when_the_state_is_missing(
    reader: PReader, registry: StateRegistry, tmp_file: Path, lookup: Callable[..., Any]
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    assert not lookup(registry, TEST_MISSING_STATE_NAME)


def test_resync_raises_when_the_name_is_a_path(
    registry: StateRegistry, tmp_file: Path
) -> None:
    with pytest.raises(TypeError, match="not an instance of 'str'"):
        registry.resync(Path(TEST_NESTED_STATE_NAME), tmp_file)  # type: ignore[arg-type]


@pytest.mark.parametrize("call", WALKS)
def test_walk_raises_when_the_state_dir_is_a_file(
    config: Config, registry: StateRegistry, call: Callable[..., Any]
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    with pytest.raises(NotADirectoryError):
        call(registry)


def test_path_joins_a_state_dir_that_is_a_file(
    registry: StateRegistry, config: Config
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    assert registry.path(TEST_STATE_NAME) == config.state_dir / TEST_STATE_FILE


def test_contains_reports_only_saved_states(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    assert TEST_STATE_NAME not in registry

    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    assert TEST_STATE_NAME in registry


@pytest.mark.parametrize("name", VALID_STATE_NAMES)
def test_resync_accepts_a_valid_name(
    reader: PReader, registry: StateRegistry, tmp_file: Path, name: str
) -> None:
    reader.bytes(tmp_file, state=name).save()

    assert registry.resync(name, tmp_file).name == name


def test_save_leaves_no_temporary_file(
    reader: PReader, config: Config, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    assert list(config.state_dir.glob("*.tmp")) == []


def test_len_counts_the_saved_states(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    assert len(registry) == 0

    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    assert len(registry) == 1


def test_exists_reports_only_saved_states(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    assert not registry.exists(TEST_STATE_NAME)

    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    assert registry.exists(TEST_STATE_NAME)


@pytest.mark.parametrize(
    "lookup", [lookup for lookup in LOOKUPS if lookup.id in {"getitem", "load"}]
)
def test_lookup_raises_when_the_state_is_a_directory(
    registry: StateRegistry, lookup: Callable[..., Any]
) -> None:
    registry.path(TEST_STATE_NAME).mkdir(parents=True)

    with pytest.raises(KeyError, match=TEST_STATE_NAME):
        lookup(registry, TEST_STATE_NAME)


def test_delitem_raises_when_the_state_is_a_directory(registry: StateRegistry) -> None:
    registry.path(TEST_STATE_NAME).mkdir(parents=True)

    with pytest.raises((IsADirectoryError, PermissionError)):
        del registry[TEST_STATE_NAME]


def test_resync_raises_when_the_state_is_unsaved(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME)

    with pytest.raises(KeyError, match=TEST_STATE_NAME):
        registry.resync(TEST_STATE_NAME, tmp_file)


@pytest.mark.parametrize("shape", [str, Path], ids=["str", "path"])
def test_resync_accepts_every_path_shape(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    shape: Callable[[Path], Any],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    assert registry.resync(TEST_STATE_NAME, shape(tmp_file)).file.path == tmp_file


def test_path_uses_the_native_separator(
    registry: StateRegistry, config: Config
) -> None:
    path = registry.path(TEST_NESTED_STATE_NAME)

    assert str(path) == str(config.state_dir / "sub-1" / "sub-2" / TEST_STATE_FILE)


@pytest.mark.parametrize("call", WALKS)
def test_walk_raises_when_the_state_dir_is_unreadable(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    revoke_permissions: Callable[..., Path],
    call: Callable[..., Any],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    revoke_permissions(registry.state_dir)

    with pytest.raises(PermissionError):
        call(registry)


def test_delitem_is_seen_by_a_registry_sharing_the_state_dir(
    reader: PReader, make_reader: Callable[..., PReader], tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    other = make_reader()

    del other.states[TEST_STATE_NAME]

    assert TEST_STATE_NAME not in reader.states


def test_resync_raises_when_the_path_is_missing(
    reader: PReader, registry: StateRegistry, tmp_file: Path, tmp_path: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    with pytest.raises(FileNotFoundError, match="os error"):
        registry.resync(TEST_STATE_NAME, tmp_path / "missing.bin")


def test_iter_yields_every_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).save()

    assert sorted(registry) == [TEST_STATE_NAME, TEST_OTHER_STATE_NAME]


def test_getitem_raises_when_the_content_is_not_utf8(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    registry.path(TEST_STATE_NAME).write_bytes(b'{"name": "\xff"}')

    with pytest.raises(StateCorruptError, match="invalid unicode"):
        registry[TEST_STATE_NAME]


def test_resync_saves_the_resynced_state(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    append: Callable[[Path, bytes], None],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    append(tmp_file, b"more")

    resynced = registry.resync(TEST_STATE_NAME, tmp_file)

    assert registry[TEST_STATE_NAME] == resynced


def test_save_raises_when_the_state_path_is_a_directory(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    iterator = reader.bytes(tmp_file)
    registry.path(iterator.state.name).mkdir(parents=True)

    with pytest.raises((IsADirectoryError, PermissionError)):
        iterator.save()


def test_find_returns_none_when_the_state_is_corrupt(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    registry.path(TEST_STATE_NAME).write_text("not valid json", encoding="utf-8")

    assert registry.exists(TEST_STATE_NAME)
    assert registry.find(TEST_STATE_NAME) is None


def test_find_returns_none_when_the_state_does_not_verify(
    reader: PReader,
    make_reader: Callable[..., PReader],
    registry: StateRegistry,
    tmp_file: Path,
    append: Callable[[Path, bytes], None],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    append(tmp_file, b"more")

    assert make_reader(verify_state=False).states.find(TEST_STATE_NAME) is not None
    assert registry.find(TEST_STATE_NAME) is None


@pytest.mark.parametrize(("payload", "message"), MALFORMED_PAYLOADS)
def test_load_raises_when_the_payload_is_malformed(
    reader: PReader, registry: StateRegistry, tmp_file: Path, payload: str, message: str
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    registry.path(TEST_STATE_NAME).write_text(payload, encoding="utf-8")

    with pytest.raises(StateCorruptError, match=message):
        registry.load(TEST_STATE_NAME)


@pytest.mark.parametrize("lookup", KEYED_LOOKUPS)
def test_lookup_raises_when_the_name_is_suffixed(
    reader: PReader, registry: StateRegistry, tmp_file: Path, lookup: Callable[..., Any]
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    state_path = registry.path(TEST_STATE_NAME)

    with pytest.raises(KeyError, match=TEST_STATE_FILE):
        lookup(registry, TEST_STATE_FILE)

    assert state_path.exists()


def test_delete_removes_the_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    state_path = registry.path(TEST_STATE_NAME)

    registry.delete(TEST_STATE_NAME)

    assert not registry.exists(TEST_STATE_NAME)
    assert not state_path.exists()


@pytest.mark.parametrize("call", WALKS)
def test_walk_raises_when_a_subdirectory_is_unreadable(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    make_stray: Callable[[str], Path],
    revoke_permissions: Callable[..., Path],
    call: Callable[..., Any],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    stray = make_stray(f"{TEST_SUB_STATE_NAME}/{TEST_OTHER_STATE_FILE}")

    revoke_permissions(stray.parent)

    with pytest.raises(PermissionError):
        call(registry)


def test_clear_removes_states_at_every_depth(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    for state in TEST_EVERY_DEPTH:
        reader.bytes(tmp_file, state=state).save()

    registry.clear()

    assert len(registry) == 0
    assert not list(config.state_dir.rglob(f"*{TEST_STATE_FILE_EXTENSION}"))


def test_clear_removes_a_corrupt_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    registry.path(TEST_OTHER_STATE_NAME).write_text("not valid json", encoding="utf-8")

    assert len(registry) == 2

    registry.clear()

    assert len(registry) == 0


def test_getitem_raises_when_a_field_has_the_wrong_type(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    with tamper(registry.path(TEST_STATE_NAME)) as payload:
        payload["position"] = "abc"

    with pytest.raises(StateCorruptError, match="invalid type"):
        registry[TEST_STATE_NAME]


def test_getitem_raises_when_the_payload_names_another_state(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    with tamper(registry.path(TEST_STATE_NAME)) as payload:
        payload["name"] = TEST_OTHER_STATE_NAME

    with pytest.raises(StateCorruptError, match="state file belongs to"):
        registry[TEST_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
@pytest.mark.parametrize("lookup", LOOKUPS)
def test_lookup_raises_when_the_state_is_a_dangling_symlink(
    registry: StateRegistry, config: Config, tmp_path: Path, lookup: Callable[..., Any]
) -> None:
    link = config.state_dir / TEST_STATE_FILE

    config.state_dir.mkdir()
    link.symlink_to(tmp_path / "missing.json")

    with pytest.raises(StateError, match="is an alias of another entry"):
        lookup(registry, TEST_STATE_NAME)

    assert link.is_symlink()


def test_getitem_raises_when_the_mtime_is_not_an_integer(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    with tamper(registry.path(TEST_STATE_NAME)) as payload:
        payload["file"]["mtime"] = "foo"

    with pytest.raises(StateCorruptError, match="invalid type"):
        registry[TEST_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
def test_resync_canonicalizes_a_symlink(
    reader: PReader,
    registry: StateRegistry,
    tmp_path: Path,
    make_file: Callable[..., Path],
) -> None:
    target = make_file(b"foo", name="real.bin")
    reader.bytes(target, state=TEST_STATE_NAME).save()

    link = tmp_path / "link.bin"
    link.symlink_to(target)

    assert registry.resync(TEST_STATE_NAME, link).file.path == target


@pytest.mark.usefixtures("requires_symlinks")
def test_resync_raises_when_the_state_is_a_dangling_symlink(
    registry: StateRegistry, config: Config, tmp_path: Path, tmp_file: Path
) -> None:
    link = config.state_dir / TEST_STATE_FILE

    config.state_dir.mkdir()
    link.symlink_to(tmp_path / "missing.json")

    with pytest.raises(StateError, match="is an alias of another entry"):
        registry.resync(TEST_STATE_NAME, tmp_file)

    assert link.is_symlink()


@pytest.mark.parametrize(
    "lookup",
    [lookup for lookup in LOOKUPS + PROBES if lookup.id in {"getitem", "load", "find"}],
)
def test_lookup_returns_the_saved_state(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    consume: Callable[..., Any],
    lookup: Callable[..., Any],
) -> None:
    saved = consume(reader.bytes(tmp_file, state=TEST_STATE_NAME), 2).save()

    state = lookup(registry, TEST_STATE_NAME)

    assert state == saved
    assert state.name == TEST_STATE_NAME
    assert state.file.path == tmp_file


def test_names_lists_the_states_saved_without_a_state_dir(
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.chdir(tmp_path)

    reader = make_reader(state_dir="")
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    assert list(reader.states.names()) == [TEST_STATE_NAME]
    assert len(reader.states) == 1


@pytest.mark.parametrize("state", TEST_EVERY_DEPTH, ids=["flat", "nested", "deep"])
def test_save_creates_the_state_file_under_its_name(
    reader: PReader,
    registry: StateRegistry,
    config: Config,
    tmp_file: Path,
    read_state: Callable[..., Any],
    state: str,
) -> None:
    saved = reader.bytes(tmp_file, state=state).save()
    path = registry.path(state)

    assert path == config.state_dir / f"{state}{TEST_STATE_FILE_EXTENSION}"
    assert path.is_file()
    assert read_state(path, saved)


@pytest.mark.usefixtures("requires_symlinks")
def test_path_raises_when_the_state_file_is_a_symlink(
    registry: StateRegistry, config: Config, tmp_path: Path
) -> None:
    outside = tmp_path / "outside.json"
    outside.write_text("{}", encoding="utf-8")

    config.state_dir.mkdir()
    (config.state_dir / TEST_STATE_FILE).symlink_to(outside)

    with pytest.raises(StateError, match="is an alias of another entry"):
        registry.path(TEST_STATE_NAME)


def test_getitem_raises_when_a_stamp_is_out_of_range(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    with tamper(registry.path(TEST_STATE_NAME)) as payload:
        payload["timestamps"]["created_at"] = 2**63 - 1

    with pytest.raises(StateCorruptError, match="not in range"):
        registry[TEST_STATE_NAME]


@pytest.mark.usefixtures("requires_symlinks")
@pytest.mark.parametrize("lookup", LOOKUPS)
def test_lookup_raises_when_the_state_is_a_symlink(
    reader: PReader,
    registry: StateRegistry,
    config: Config,
    tmp_file: Path,
    lookup: Callable[..., Any],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    real = registry.path(TEST_STATE_NAME)

    alias = config.state_dir / TEST_OTHER_STATE_FILE
    alias.symlink_to(real)

    with pytest.raises(StateError, match="is an alias of another entry"):
        lookup(registry, TEST_OTHER_STATE_NAME)


def test_find_returns_none_when_the_payload_names_another_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    saved = registry.path(TEST_STATE_NAME).read_text(encoding="utf-8")

    registry.path(TEST_OTHER_STATE_NAME).write_text(saved, encoding="utf-8")

    assert registry.exists(TEST_OTHER_STATE_NAME)
    assert registry.find(TEST_OTHER_STATE_NAME) is None


def test_resync_canonicalizes_an_unnormalized_path(
    reader: PReader,
    registry: StateRegistry,
    tmp_path: Path,
    make_file: Callable[..., Path],
) -> None:
    target = make_file(b"foo", name="real.bin")
    reader.bytes(target, state=TEST_STATE_NAME).save()

    (tmp_path / "folder").mkdir()
    detour = tmp_path / "folder" / ".." / "real.bin"

    assert registry.resync(TEST_STATE_NAME, detour).file.path == target


@pytest.mark.usefixtures("requires_symlinks")
def test_names_ignores_a_directory_alias(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=f"real/{TEST_STATE_NAME}").save()

    state_path = registry.path(f"real/{TEST_STATE_NAME}")

    (config.state_dir / "alias").symlink_to(state_path.parent, target_is_directory=True)

    assert list(registry.names()) == [f"real/{TEST_STATE_NAME}"]


def test_len_includes_states_that_all_rejects(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    registry.path(TEST_OTHER_STATE_NAME).write_text("not valid json", encoding="utf-8")

    assert len(registry) == 2

    with pytest.raises(StateCorruptError, match="expected ident") as exc_info:
        registry.all()

    assert exc_info.value.name == TEST_OTHER_STATE_NAME


def test_save_leaves_no_temporary_file_when_it_fails(
    reader: PReader, config: Config, tmp_file: Path
) -> None:
    config.state_dir.mkdir()
    (config.state_dir / TEST_STATE_FILE).mkdir()

    iterator = reader.bytes(tmp_file, state=TEST_STATE_NAME)

    with pytest.raises((IsADirectoryError, PermissionError)):
        iterator.save()

    assert list(config.state_dir.glob("*.tmp")) == []


@pytest.mark.parametrize(
    "corrupted",
    [TEST_STATE_NAME, TEST_OTHER_STATE_NAME, "job-3"],
    ids=["first", "middle", "last"],
)
def test_all_raises_when_any_state_is_corrupt(
    reader: PReader, registry: StateRegistry, tmp_file: Path, corrupted: str
) -> None:
    for name in (TEST_STATE_NAME, TEST_OTHER_STATE_NAME, "job-3"):
        reader.bytes(tmp_file, state=name).save()

    registry.path(corrupted).write_text("not valid json", encoding="utf-8")

    with pytest.raises(StateCorruptError, match="expected ident") as exc_info:
        registry.all()

    assert exc_info.value.name == corrupted


@pytest.mark.parametrize("lookup", LOOKUPS)
@pytest.mark.parametrize(("name", "alias"), ALIAS_STATE_NAMES)
def test_lookup_raises_when_the_name_is_a_case_or_unicode_alias(
    reader: PReader,
    registry: StateRegistry,
    config: Config,
    tmp_file: Path,
    lookup: Callable[..., Any],
    name: str,
    alias: str,
) -> None:
    reader.bytes(tmp_file, state=name).save()

    if not (config.state_dir / f"{alias}{TEST_STATE_FILE_EXTENSION}").exists():
        pytest.skip("the file system tells the two names apart")

    with pytest.raises(StateError, match="is an alias of another entry"):
        lookup(registry, alias)

    assert name in registry.names()


@pytest.mark.usefixtures("requires_symlinks")
def test_resync_raises_when_the_state_is_a_symlink(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    real = registry.path(TEST_STATE_NAME)

    alias = config.state_dir / TEST_OTHER_STATE_FILE
    alias.symlink_to(real)

    with pytest.raises(StateError, match="is an alias of another entry"):
        registry.resync(TEST_OTHER_STATE_NAME, tmp_file)

    assert alias.is_symlink()


def test_delitem_removes_only_the_hard_link(
    registry: StateRegistry, config: Config, tmp_path: Path
) -> None:
    outside = tmp_path / "outside.txt"
    outside.write_text("foo", encoding="utf-8")

    link = config.state_dir / TEST_STATE_FILE
    config.state_dir.mkdir()
    link.hardlink_to(outside)

    del registry[TEST_STATE_NAME]

    assert not link.exists()
    assert outside.read_text(encoding="utf-8") == "foo"


@pytest.mark.parametrize(("name", "alias"), ALIAS_STATE_NAMES)
def test_resync_raises_when_the_name_is_a_case_or_unicode_alias(
    reader: PReader,
    registry: StateRegistry,
    config: Config,
    tmp_file: Path,
    name: str,
    alias: str,
) -> None:
    reader.bytes(tmp_file, state=name).save()

    if not (config.state_dir / f"{alias}{TEST_STATE_FILE_EXTENSION}").exists():
        pytest.skip("the file system tells the two names apart")

    with pytest.raises(StateError, match="is an alias of another entry"):
        registry.resync(alias, tmp_file)

    assert name in registry.names()


def test_save_raises_when_a_directory_blocks_the_state_file(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=f"{TEST_STATE_FILE}/{TEST_OTHER_STATE_NAME}").save()

    nested = registry.path(f"{TEST_STATE_FILE}/{TEST_OTHER_STATE_NAME}")
    iterator = reader.bytes(tmp_file, state=TEST_STATE_NAME)

    with pytest.raises(OSError, match="os error"):
        iterator.save()

    assert nested.exists()


def test_path_is_relative_without_a_state_dir(
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    read_state: Callable[..., Any],
) -> None:
    monkeypatch.chdir(tmp_path)

    reader = make_reader(state_dir="")
    saved = reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    path = reader.states.path(TEST_STATE_NAME)

    assert str(path) == TEST_STATE_FILE
    assert (tmp_path / path).exists()
    assert read_state(path, saved)


@pytest.mark.usefixtures("requires_symlinks")
def test_path_raises_when_the_state_is_a_symlink_without_a_state_dir(
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.chdir(tmp_path)

    reader = make_reader(state_dir="")
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    real = reader.states.path(TEST_STATE_NAME)

    Path(TEST_OTHER_STATE_FILE).symlink_to(real)

    with pytest.raises(StateError, match="is an alias of another entry"):
        reader.states.path(TEST_OTHER_STATE_NAME)


@pytest.mark.usefixtures("requires_symlinks")
def test_resync_raises_when_the_path_is_a_symlink_loop(
    reader: PReader, registry: StateRegistry, tmp_file: Path, tmp_path: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    first = tmp_path / "link-1.bin"
    second = tmp_path / "link-2.bin"

    first.symlink_to(second)
    second.symlink_to(first)

    with pytest.raises(
        OSError,
        match="cannot be resolved" if sys.platform == "win32" else "Too many levels",
    ):
        registry.resync(TEST_STATE_NAME, first)


def test_resync_saves_the_state_when_the_file_is_unchanged(
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    reader = make_reader(verify_state=False)
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["timestamps"] = {"created_at": 0, "updated_at": 0}

    resynced = reader.states.resync(TEST_STATE_NAME, tmp_file)

    assert make_reader().states[TEST_STATE_NAME] == resynced


def test_getitem_ignores_an_unknown_field(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    consume: Callable[..., Any],
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    saved = consume(reader.bytes(tmp_file, state=TEST_STATE_NAME), 2).save()

    with tamper(registry.path(TEST_STATE_NAME)) as payload:
        payload["foo"] = 42

    state = registry[TEST_STATE_NAME]

    assert state == saved
    assert state.name == TEST_STATE_NAME
    assert state.file.path == tmp_file


def test_resync_raises_when_the_state_cannot_be_saved(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    append: Callable[[Path, bytes], None],
    revoke_permissions: Callable[..., Path],
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    state_path = registry.path(TEST_STATE_NAME)
    before = state_path.read_bytes()

    append(tmp_file, b"more")

    revoke_permissions(registry.state_dir, 0o500)

    with pytest.raises(PermissionError):
        registry.resync(TEST_STATE_NAME, tmp_file)

    assert state_path.read_bytes() == before


def test_save_raises_when_a_state_file_blocks_the_directory(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    state_path = registry.path(TEST_STATE_NAME)
    before = state_path.read_bytes()
    iterator = reader.bytes(
        tmp_file, state=f"{TEST_STATE_FILE}/{TEST_OTHER_STATE_NAME}"
    )

    with pytest.raises(OSError, match="os error"):
        iterator.save()

    assert state_path.read_bytes() == before


@pytest.mark.usefixtures("requires_symlinks")
@pytest.mark.parametrize("lookup", PROBES)
def test_lookup_answers_when_the_state_is_a_symlink(
    make_reader: Callable[..., PReader],
    config: Config,
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    lookup: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    real = reader.states.path(TEST_STATE_NAME)

    with tamper(real) as payload:
        payload["name"] = TEST_OTHER_STATE_NAME

    alias = config.state_dir / TEST_OTHER_STATE_FILE
    alias.symlink_to(real)

    assert not lookup(reader.states, TEST_OTHER_STATE_NAME)


@pytest.mark.usefixtures("requires_symlinks")
@pytest.mark.parametrize("lookup", LOOKUPS)
def test_lookup_raises_when_the_name_escapes_through_a_symlink(
    registry: StateRegistry, config: Config, tmp_path: Path, lookup: Callable[..., Any]
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir()

    config.state_dir.mkdir()
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    victim = outside / TEST_STATE_FILE
    victim.write_text("{}", encoding="utf-8")

    with pytest.raises(StateError, match="is an alias of another entry"):
        lookup(registry, f"link/{TEST_STATE_NAME}")

    assert victim.is_file()


@pytest.mark.parametrize("lookup", PROBES)
@pytest.mark.parametrize(("name", "alias"), ALIAS_STATE_NAMES)
def test_lookup_answers_when_the_name_is_a_case_or_unicode_alias(
    make_reader: Callable[..., PReader],
    config: Config,
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    lookup: Callable[..., Any],
    name: str,
    alias: str,
) -> None:
    reader = make_reader(verify_state=False)
    reader.bytes(tmp_file, state=name).save()

    with tamper(reader.states.path(name)) as payload:
        payload["name"] = alias

    if not (config.state_dir / f"{alias}{TEST_STATE_FILE_EXTENSION}").exists():
        pytest.skip("the file system tells the two names apart")

    assert not lookup(reader.states, alias)


def test_save_replaces_a_hard_link_without_touching_its_target(
    reader: PReader,
    config: Config,
    tmp_file: Path,
    tmp_path: Path,
    read_state: Callable[..., Any],
) -> None:
    iterator = reader.bytes(tmp_file, state=TEST_STATE_NAME)

    outside = tmp_path / "outside.txt"
    outside.write_text("foo", encoding="utf-8")

    link = config.state_dir / TEST_STATE_FILE
    config.state_dir.mkdir()
    link.hardlink_to(outside)

    saved = iterator.save()

    assert outside.read_text(encoding="utf-8") == "foo"
    assert read_state(link, saved)


@pytest.mark.usefixtures("requires_symlinks")
@pytest.mark.parametrize("lookup", LOOKUPS)
def test_lookup_raises_when_the_name_is_a_directory_alias(
    reader: PReader,
    registry: StateRegistry,
    config: Config,
    tmp_file: Path,
    lookup: Callable[..., Any],
) -> None:
    reader.bytes(tmp_file, state=f"real/{TEST_STATE_NAME}").save()

    state_path = registry.path(f"real/{TEST_STATE_NAME}")
    before = state_path.read_bytes()

    (config.state_dir / "alias").symlink_to(state_path.parent, target_is_directory=True)

    with pytest.raises(StateError, match="is an alias of another entry"):
        lookup(registry, f"alias/{TEST_STATE_NAME}")

    assert state_path.read_bytes() == before


def test_clear_removes_every_saved_state(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()
    first = registry.path(TEST_STATE_NAME)

    reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME).save()
    second = registry.path(TEST_OTHER_STATE_NAME)

    assert len(registry) == 2

    registry.clear()

    assert len(registry) == 0
    assert not first.exists()
    assert not second.exists()


@pytest.mark.usefixtures("requires_symlinks")
def test_resync_raises_when_the_name_is_a_directory_alias(
    reader: PReader, registry: StateRegistry, config: Config, tmp_file: Path
) -> None:
    reader.bytes(tmp_file, state=f"real/{TEST_STATE_NAME}").save()

    state_path = registry.path(f"real/{TEST_STATE_NAME}")
    before = state_path.read_bytes()

    (config.state_dir / "alias").symlink_to(state_path.parent, target_is_directory=True)

    with pytest.raises(StateError, match="is an alias of another entry"):
        registry.resync(f"alias/{TEST_STATE_NAME}", tmp_file)

    assert state_path.read_bytes() == before


@pytest.mark.usefixtures("requires_symlinks")
def test_resync_raises_when_the_name_escapes_through_a_symlink(
    registry: StateRegistry, config: Config, tmp_path: Path, tmp_file: Path
) -> None:
    outside = tmp_path / "outside"
    outside.mkdir()

    config.state_dir.mkdir()
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    victim = outside / TEST_STATE_FILE
    victim.write_text("{}", encoding="utf-8")

    with pytest.raises(StateError, match="is an alias of another entry"):
        registry.resync(f"link/{TEST_STATE_NAME}", tmp_file)

    assert victim.read_text(encoding="utf-8") == "{}"


@pytest.mark.usefixtures("requires_symlinks")
@pytest.mark.parametrize("lookup", PROBES)
def test_lookup_answers_when_the_name_is_a_directory_alias(
    make_reader: Callable[..., PReader],
    config: Config,
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
    lookup: Callable[..., Any],
) -> None:
    reader = make_reader(verify_state=False)
    reader.bytes(tmp_file, state=f"real/{TEST_STATE_NAME}").save()

    state_path = reader.states.path(f"real/{TEST_STATE_NAME}")

    with tamper(state_path) as payload:
        payload["name"] = f"alias/{TEST_STATE_NAME}"

    (config.state_dir / "alias").symlink_to(state_path.parent, target_is_directory=True)

    assert not lookup(reader.states, f"alias/{TEST_STATE_NAME}")


@pytest.mark.parametrize(("name", "alias"), ALIAS_STATE_NAMES)
def test_delitem_never_touches_the_state_behind_an_alias(
    reader: PReader, registry: StateRegistry, tmp_file: Path, name: str, alias: str
) -> None:
    reader.bytes(tmp_file, state=name).save()

    state_path = registry.path(name)
    before = state_path.read_bytes()

    with contextlib.suppress(KeyError, StateError):
        del registry[alias]

    with contextlib.suppress(StateError, OSError):
        reader.bytes(tmp_file, state=alias).save()

    assert name in registry.names()
    assert state_path.read_bytes() == before


@pytest.mark.usefixtures("requires_symlinks")
@pytest.mark.parametrize("lookup", PROBES)
def test_lookup_answers_when_the_name_escapes_through_a_symlink(
    make_reader: Callable[..., PReader],
    config: Config,
    tmp_path: Path,
    tmp_file: Path,
    lookup: Callable[..., Any],
) -> None:
    name = f"link/{TEST_STATE_NAME}"
    reader = make_reader(verify_state=False)
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    payload = json.loads(
        reader.states.path(TEST_STATE_NAME).read_text(encoding="utf-8")
    )

    outside = tmp_path / "outside"
    outside.mkdir()

    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    (outside / TEST_STATE_FILE).write_text(
        json.dumps({**payload, "name": name}), encoding="utf-8"
    )

    assert not lookup(reader.states, name)


def test_all_returns_every_saved_state(
    reader: PReader,
    registry: StateRegistry,
    tmp_file: Path,
    consume: Callable[..., Any],
) -> None:
    first = consume(reader.bytes(tmp_file, state=TEST_STATE_NAME), 2).save()
    second = consume(reader.bytes(tmp_file, state=TEST_OTHER_STATE_NAME), 3).save()

    states = sorted(registry.all(), key=lambda state: state.name)

    assert [(state.name, state.position, state.file.path) for state in states] == [
        (TEST_STATE_NAME, first.position, tmp_file),
        (TEST_OTHER_STATE_NAME, second.position, tmp_file),
    ]
    assert states == [first, second]


@pytest.mark.usefixtures("requires_symlinks")
def test_names_uses_a_symlinked_state_dir_as_the_root(
    make_reader: Callable[..., PReader], tmp_file: Path, tmp_path: Path
) -> None:
    real = tmp_path / "real"
    real.mkdir()

    link = tmp_path / "link"
    link.symlink_to(real, target_is_directory=True)

    reader = make_reader(state_dir=link)
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    assert (real / TEST_STATE_FILE).is_file()
    assert list(reader.states.names()) == [TEST_STATE_NAME]
    assert reader.states[TEST_STATE_NAME].name == TEST_STATE_NAME

    del reader.states[TEST_STATE_NAME]

    assert list(real.iterdir()) == []


@pytest.mark.parametrize("lookup", PROBES)
@pytest.mark.parametrize("name", INVALID_STATE_NAMES)
def test_lookup_answers_when_the_name_is_invalid(
    make_reader: Callable[..., PReader],
    tmp_path: Path,
    tmp_file: Path,
    lookup: Callable[..., Any],
    name: str,
) -> None:
    reader = make_reader(state_dir=tmp_path / "nested" / "preader", verify_state=False)
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    payload = json.loads(
        reader.states.path(TEST_STATE_NAME).read_text(encoding="utf-8")
    )
    target = reader.states.state_dir.joinpath(*name.split("/"))

    if target.is_relative_to(tmp_path):
        stray = target.with_name(f"{target.name}{TEST_STATE_FILE_EXTENSION}")

        with contextlib.suppress(OSError, ValueError):
            stray.parent.mkdir(parents=True, exist_ok=True)
            stray.write_text(json.dumps({**payload, "name": name}), encoding="utf-8")

    assert not lookup(reader.states, name)


@pytest.mark.parametrize("name", VALID_STATE_NAMES)
def test_saved_name_round_trips_through_names(
    reader: PReader,
    registry: StateRegistry,
    config: Config,
    tmp_file: Path,
    read_state: Callable[..., Any],
    name: str,
) -> None:
    path = config.state_dir / f"{name}{TEST_STATE_FILE_EXTENSION}"
    state = reader.bytes(tmp_file, state=name).save()

    assert registry.path(name) == path
    assert read_state(path, state)
    assert list(registry.names()) == [name]
    assert len(registry) == 1
    assert list(registry.search(f"^{re.escape(name)}$")) == [name]
    assert registry[name] == state

    del registry[name]

    assert not path.exists()

    reader.bytes(tmp_file, state=name).save()
    registry.clear()

    assert not path.exists()


def test_lookup_raises_when_the_name_is_a_short_name_alias(
    reader: PReader, registry: StateRegistry, tmp_file: Path
) -> None:
    if sys.platform != "win32":
        pytest.skip("8.3 short names exist only on Windows")

    reader.bytes(tmp_file, state=f"longdirectoryname/{TEST_STATE_NAME}").save()

    state_path = registry.path(f"longdirectoryname/{TEST_STATE_NAME}")
    before = state_path.read_bytes()
    buffer = ctypes.create_unicode_buffer(32768)

    ctypes.windll.kernel32.GetShortPathNameW(
        str(state_path.parent), buffer, len(buffer)
    )

    short = Path(buffer.value).name

    if short.lower() == state_path.parent.name.lower():
        pytest.skip("8.3 short names are disabled on this volume")

    with pytest.raises(StateError, match="is an alias of another entry"):
        registry[f"{short}/{TEST_STATE_NAME}"]

    with pytest.raises(StateError, match="is an alias of another entry"):
        reader.bytes(tmp_file, state=f"{short}/{TEST_STATE_NAME}").save()

    assert list(registry.names()) == [f"longdirectoryname/{TEST_STATE_NAME}"]
    assert state_path.read_bytes() == before
