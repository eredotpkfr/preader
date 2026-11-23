import json
import os

from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

from common.constants import (
    TEST_LINE,
    TEST_OTHER_STATE_NAME,
    TEST_STATE_NAME,
    TEST_TRACKED_NAME,
    TEST_UNTRACKED_NAME,
)
from preader import (
    Config,
    IteratorOptions,
    PReader,
    StateConflictError,
    StateCorruptError,
    StateError,
    StateMismatchError,
    StateOutdatedError,
    StateRegistry,
)

type Trigger = Callable[[], object]


def _moved(reader: PReader, path: Path) -> Trigger:
    moved = path.rename(path.with_name("moved.bin"))

    return lambda: reader.bytes(moved, state=TEST_STATE_NAME)


def _grown(reader: PReader, path: Path) -> Trigger:
    with path.open("ab") as file:
        file.write(b"more")

    return lambda: reader.states[TEST_STATE_NAME]


def _touched(reader: PReader, path: Path) -> Trigger:
    mtime = path.stat().st_mtime_ns + 3_600_000_000_000

    os.utime(path, ns=(mtime, mtime))

    return lambda: reader.states[TEST_STATE_NAME]


def _deleted(reader: PReader, path: Path) -> Trigger:
    path.unlink()

    return lambda: reader.states[TEST_STATE_NAME]


def _replaced(reader: PReader, path: Path) -> Trigger:
    untracked = path.with_name(TEST_UNTRACKED_NAME)
    untracked.write_bytes(b"bar\n")

    return lambda: reader.states.resync(TEST_STATE_NAME, untracked)


def _unsaved(reader: PReader, path: Path) -> Trigger:
    iterator = reader.bytes(path, state=TEST_STATE_NAME)

    next(iterator)
    next(iterator)

    state = iterator.state

    return lambda: reader.bytes(path, state=state)


def _unparsable(reader: PReader, _path: Path) -> Trigger:
    reader.states.path(TEST_STATE_NAME).write_text("not valid json", encoding="utf-8")

    return lambda: reader.states[TEST_STATE_NAME]


def _tampered(reader: PReader, _path: Path) -> Trigger:
    saved = reader.states.path(TEST_STATE_NAME)
    content = saved.read_text(encoding="utf-8")

    payload: dict[str, Any] = json.loads(content)
    payload["position"] = 1

    saved.write_text(json.dumps(payload), encoding="utf-8")

    return lambda: reader.states[TEST_STATE_NAME]


def _misplaced(reader: PReader, _path: Path) -> Trigger:
    saved = reader.states.path(TEST_STATE_NAME).read_text(encoding="utf-8")
    reader.states.path(TEST_OTHER_STATE_NAME).write_text(saved, encoding="utf-8")

    return lambda: reader.states[TEST_OTHER_STATE_NAME]


def _overtaken(reader: PReader, path: Path) -> Trigger:
    first = reader.bytes(path, state=TEST_STATE_NAME)
    second = reader.bytes(path, state=TEST_STATE_NAME)

    next(first)
    next(first)

    first.save()

    return second.save


STATE_ERROR_CAUSES = [
    pytest.param(
        _moved, StateOutdatedError, "file path changed", TEST_STATE_NAME, id="moved"
    ),
    pytest.param(
        _grown, StateOutdatedError, "file size changed", TEST_STATE_NAME, id="grown"
    ),
    pytest.param(
        _touched,
        StateOutdatedError,
        "file mtime changed",
        TEST_STATE_NAME,
        id="touched",
    ),
    pytest.param(
        _deleted, StateOutdatedError, "file is missing", TEST_STATE_NAME, id="deleted"
    ),
    pytest.param(
        _replaced,
        StateMismatchError,
        "file is not the tracked file",
        TEST_STATE_NAME,
        id="replaced",
    ),
    pytest.param(
        _unsaved,
        StateMismatchError,
        "has unsaved progress",
        TEST_STATE_NAME,
        id="unsaved",
    ),
    pytest.param(
        _unparsable,
        StateCorruptError,
        "state file is not valid",
        TEST_STATE_NAME,
        id="unparsable",
    ),
    pytest.param(
        _tampered,
        StateCorruptError,
        "modified outside preader",
        TEST_STATE_NAME,
        id="tampered",
    ),
    pytest.param(
        _misplaced,
        StateCorruptError,
        f"belongs to '{TEST_STATE_NAME}'",
        TEST_OTHER_STATE_NAME,
        id="misplaced",
    ),
    pytest.param(
        _overtaken,
        StateConflictError,
        "saved elsewhere",
        TEST_STATE_NAME,
        id="overtaken",
    ),
]


def test_regex_errors_map_to_value_error(registry: StateRegistry) -> None:
    with pytest.raises(ValueError, match="regex parse error"):
        registry.search("[invalid(")


def test_directories_map_to_is_a_directory_error(
    reader: PReader, tmp_path: Path
) -> None:
    with pytest.raises(IsADirectoryError, match="is a directory"):
        reader.bytes(tmp_path)


def test_missing_states_map_to_key_error(registry: StateRegistry) -> None:
    with pytest.raises(KeyError, match=TEST_STATE_NAME):
        registry[TEST_STATE_NAME]


def test_io_errors_map_to_os_errors(reader: PReader, tmp_path: Path) -> None:
    with pytest.raises(FileNotFoundError, match="os error"):
        reader.bytes(tmp_path / "missing.bin")


def test_state_dir_files_map_to_not_a_directory_error(
    config: Config, registry: StateRegistry
) -> None:
    config.state_dir.write_bytes(b"foo")

    with pytest.raises(NotADirectoryError, match="not a directory"):
        len(registry)


def test_invalid_ranges_map_to_value_error(reader: PReader, tmp_file: Path) -> None:
    with pytest.raises(ValueError, match="start must not be greater than end"):
        reader.bytes(tmp_file, options=IteratorOptions(start=9, end=1))


def test_name_errors_carry_the_bare_message(registry: StateRegistry) -> None:
    with pytest.raises(ValueError, match="escapes the state directory") as exc_info:
        registry.path("../../etc/passwd")

    assert str(exc_info.value) == "state name escapes the state directory"


@pytest.mark.parametrize(("cause", "error", "message", "name"), STATE_ERROR_CAUSES)
def test_causes_map_to_their_state_error(
    reader: PReader,
    tmp_file: Path,
    cause: Callable[[PReader, Path], Trigger],
    error: type[StateError],
    message: str,
    name: str,
) -> None:
    reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    trigger = cause(reader, tmp_file)

    with pytest.raises(error, match=message) as exc_info:
        trigger()

    assert type(exc_info.value) is error
    assert exc_info.value.name == name
    assert str(exc_info.value).startswith(f"state '{name}': ")


def test_all_names_the_failing_state(
    reader: PReader,
    make_file: Callable[..., Path],
    append: Callable[[Path, bytes], None],
) -> None:
    tracked = make_file(TEST_LINE, name=TEST_TRACKED_NAME)
    grown = make_file(TEST_LINE, name="grown.bin")

    reader.bytes(tracked, state=TEST_STATE_NAME).save()
    reader.bytes(grown, state=TEST_OTHER_STATE_NAME).save()

    append(grown, b"more")

    with pytest.raises(StateOutdatedError, match="file size changed") as exc_info:
        reader.states.all()

    assert exc_info.value.name == TEST_OTHER_STATE_NAME


@pytest.mark.usefixtures("requires_symlinks")
def test_aliases_map_to_state_error(
    registry: StateRegistry, config: Config, tmp_path: Path
) -> None:
    name = f"link/{TEST_STATE_NAME}"

    outside = tmp_path / "outside"
    outside.mkdir()

    config.state_dir.mkdir()
    (config.state_dir / "link").symlink_to(outside, target_is_directory=True)

    with pytest.raises(StateError, match="is an alias of another entry") as exc_info:
        registry.path(name)

    assert type(exc_info.value) is StateError
    assert exc_info.value.name == name


def test_resync_accepts_the_error_name_of_an_auto_named_state(
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    consume: Callable[..., Any],
    append: Callable[[Path, bytes], None],
) -> None:
    reader = make_reader(auto_load_state=True)

    iterator = reader.bytes(tmp_file)
    saved = consume(iterator, 2).save()

    append(tmp_file, b"more")

    with pytest.raises(StateOutdatedError) as exc_info:
        reader.bytes(tmp_file)

    reader.states.resync(exc_info.value.name, tmp_file)

    assert reader.bytes(tmp_file).state.position == saved.position
