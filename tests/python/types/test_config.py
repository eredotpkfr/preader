from collections.abc import Callable
from pathlib import Path

import pytest

from preader import Config


def test_config_defaults() -> None:
    config = Config()

    assert config.buffer_capacity == 64 * 1024
    assert config.auto_save_state is False
    assert config.auto_save_state_bytes == 100 * 1024 * 1024
    assert config.auto_load_state is False
    assert config.verify_state is True
    assert config.state_dir.name == "preader"
    assert config.state_dir.is_absolute()


@pytest.mark.parametrize(
    ("auto_save_state", "auto_load_state"),
    [(True, False), (False, True)],
    ids=["auto_save", "auto_load"],
)
def test_config_fields(
    tmp_path: Path, auto_save_state: bool, auto_load_state: bool
) -> None:
    config = Config(
        state_dir=tmp_path,
        buffer_capacity=1024,
        auto_save_state=auto_save_state,
        auto_save_state_bytes=2048,
        auto_load_state=auto_load_state,
        verify_state=False,
    )

    assert config.state_dir == tmp_path
    assert config.buffer_capacity == 1024
    assert config.auto_save_state is auto_save_state
    assert config.auto_save_state_bytes == 2048
    assert config.auto_load_state is auto_load_state
    assert config.verify_state is False


def test_config_compares_by_value(tmp_path: Path) -> None:
    config = Config(state_dir=tmp_path, buffer_capacity=1024)

    assert config == Config(state_dir=tmp_path, buffer_capacity=1024)
    assert config != Config(state_dir=tmp_path, buffer_capacity=2048)


def test_config_repr(tmp_path: Path, expected_repr: Callable[..., str]) -> None:
    config = Config(
        state_dir=tmp_path / "preader",
        buffer_capacity=1024,
        auto_save_state=True,
        auto_save_state_bytes=2048,
        auto_load_state=False,
        verify_state=True,
    )

    assert repr(config) == expected_repr(
        "Config",
        state_dir=config.state_dir,
        buffer_capacity=config.buffer_capacity,
        auto_save_state=config.auto_save_state,
        auto_save_state_bytes=config.auto_save_state_bytes,
        auto_load_state=config.auto_load_state,
        verify_state=config.verify_state,
    )


def test_config_raises_when_an_argument_is_positional() -> None:
    with pytest.raises(TypeError, match="positional"):
        Config(65536)  # type: ignore[call-arg]


def test_config_raises_when_the_auto_save_state_bytes_is_zero() -> None:
    with pytest.raises(ValueError, match="invalid zero value"):
        Config(auto_save_state_bytes=0)


@pytest.mark.parametrize(
    "make",
    [
        lambda value: Config(buffer_capacity=value),
        lambda value: Config(auto_save_state_bytes=value),
    ],
    ids=["buffer_capacity", "auto_save_state_bytes"],
)
@pytest.mark.parametrize(
    ("value", "message"),
    [(-1, "negative int"), (2**64, "too big")],
    ids=["negative", "too_large"],
)
def test_config_raises_when_a_size_overflows(
    make: Callable[[int], Config], value: int, message: str
) -> None:
    with pytest.raises(OverflowError, match=message):
        make(value)


@pytest.mark.parametrize(
    "field",
    [
        "auto_load_state",
        "auto_save_state",
        "auto_save_state_bytes",
        "buffer_capacity",
        "state_dir",
        "verify_state",
    ],
)
def test_config_raises_when_a_field_is_set(field: str) -> None:
    config = Config()

    with pytest.raises(AttributeError, match="is not writable"):
        setattr(config, field, None)
