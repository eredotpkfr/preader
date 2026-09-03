from collections.abc import Callable
from pathlib import Path

import pytest

from preader import Config


def test_config_raises_when_positional() -> None:
    with pytest.raises(TypeError):
        Config(65536)  # type: ignore[call-arg]


def test_config_defaults() -> None:
    config = Config()

    assert config.buffer_capacity == 64 * 1024
    assert config.auto_save_state is False
    assert config.auto_save_state_bytes == 100 * 1024 * 1024
    assert config.auto_load_state is False
    assert config.verify_state is True


def test_config_compares_by_value(tmp_path: Path) -> None:
    config = Config(state_dir=tmp_path, buffer_capacity=1024)

    assert config == Config(state_dir=tmp_path, buffer_capacity=1024)
    assert config != Config(state_dir=tmp_path, buffer_capacity=2048)


def test_config_repr(tmp_path: Path, expected_repr: Callable[..., str]) -> None:
    config = Config(
        state_dir=tmp_path / "preader-test",
        buffer_capacity=1024,
        auto_save_state=True,
        auto_save_state_bytes=2048,
        auto_load_state=False,
        verify_state=True,
    )

    assert repr(config) == expected_repr(
        "Config",
        state_dir=f"'{config.state_dir}'",
        buffer_capacity=config.buffer_capacity,
        auto_save_state=str(config.auto_save_state).lower(),
        auto_save_state_bytes=config.auto_save_state_bytes,
        auto_load_state=str(config.auto_load_state).lower(),
        verify_state=str(config.verify_state).lower(),
    )
