from pathlib import Path

import pytest

from constants import TEST_STATE_NAME
from preader import IteratorOptions, PReader, StateError, StateRegistry


def test_io_errors_map_to_os_errors(reader: PReader, tmp_file: Path) -> None:
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state

    state.save()
    tmp_file.unlink()

    with pytest.raises(FileNotFoundError):
        state.verify()


def test_missing_states_map_to_key_error(registry: StateRegistry) -> None:
    with pytest.raises(KeyError, match=TEST_STATE_NAME):
        registry[TEST_STATE_NAME]


def test_invalid_ranges_map_to_value_error(reader: PReader, tmp_file: Path) -> None:
    with pytest.raises(ValueError, match=r"start .* must be <= end"):
        reader.bytes(tmp_file, options=IteratorOptions(start=9, end=1))


def test_serde_errors_map_to_state_error(registry: StateRegistry) -> None:
    path = registry.path(TEST_STATE_NAME)

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("not valid json", encoding="utf-8")

    with pytest.raises(StateError, match="expected ident"):
        registry[TEST_STATE_NAME]


def test_regex_errors_map_to_state_error(registry: StateRegistry) -> None:
    with pytest.raises(StateError, match="regex parse error"):
        registry.search("[invalid(")


def test_path_errors_carry_the_bare_message(registry: StateRegistry) -> None:
    with pytest.raises(StateError) as exc_info:
        registry.path("../../etc/passwd")

    assert str(exc_info.value) == "path escapes root: ../../etc/passwd"
