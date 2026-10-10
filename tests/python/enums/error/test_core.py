from collections.abc import Callable
from pathlib import Path

import pytest

from constants import TEST_STATE_NAME
from preader import (
    Config,
    IteratorOptions,
    PReader,
    StateError,
    StateMismatchError,
    StateRegistry,
)


def test_io_errors_map_to_os_errors(reader: PReader, tmp_file: Path) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state

    state.save()
    tmp_file.unlink()

    with pytest.raises(FileNotFoundError):
        state.verify()


def test_directories_map_to_is_a_directory_error(
    reader: PReader, tmp_path: Path
) -> None:
    with pytest.raises(IsADirectoryError, match="is a directory"):
        reader.bytes(tmp_path)


def test_state_dir_files_map_to_not_a_directory_error(
    config: Config, registry: StateRegistry
) -> None:
    config.state_dir.write_bytes(b"not a directory")

    with pytest.raises(NotADirectoryError, match="not a directory"):
        len(registry)


def test_corrupt_states_name_the_state(registry: StateRegistry) -> None:
    path = registry.path(TEST_STATE_NAME)

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("not valid json", encoding="utf-8")

    with pytest.raises(StateError, match=f"state '{TEST_STATE_NAME}' is corrupt"):
        registry[TEST_STATE_NAME]


def test_missing_states_map_to_key_error(registry: StateRegistry) -> None:
    with pytest.raises(KeyError, match=TEST_STATE_NAME):
        registry[TEST_STATE_NAME]


def test_mismatches_map_to_state_mismatch_error(
    reader: PReader, tmp_file: Path, append: Callable[[Path, bytes], None]
) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state
    state.save()

    append(tmp_file, b"more")

    with pytest.raises(StateMismatchError, match="file size changed"):
        state.verify()


def test_state_mismatch_error_is_a_state_error() -> None:
    assert issubclass(StateMismatchError, StateError)


def test_invalid_ranges_map_to_value_error(reader: PReader, tmp_file: Path) -> None:
    with pytest.raises(ValueError, match="start must not be greater than end"):
        reader.bytes(tmp_file, options=IteratorOptions(start=9, end=1))


def test_regex_errors_map_to_value_error(registry: StateRegistry) -> None:
    with pytest.raises(ValueError, match="regex parse error"):
        registry.search("[invalid(")


def test_name_errors_carry_the_bare_message(registry: StateRegistry) -> None:
    with pytest.raises(ValueError, match="escapes the state directory") as exc_info:
        registry.path("../../etc/passwd")

    assert str(exc_info.value) == "state name escapes the state directory"
