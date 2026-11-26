from collections.abc import Callable

import pytest

from common.constants import TEST_STATE_NAME
from preader import Config, PReader


def test_preader_defaults() -> None:
    assert PReader().config == Config()


def test_preader_fields(reader: PReader, config: Config) -> None:
    assert reader.config == config


def test_preader_repr(
    reader: PReader, config: Config, expected_repr: Callable[..., str]
) -> None:
    assert repr(reader) == expected_repr("PReader", config=config)


def test_preader_raises_when_the_config_is_none() -> None:
    with pytest.raises(TypeError, match="is not an instance of 'Config'"):
        PReader(config=None)  # type: ignore[arg-type]


def test_preader_raises_when_the_config_is_positional() -> None:
    with pytest.raises(TypeError, match="positional"):
        PReader(Config())  # type: ignore[call-arg]


def test_preader_states_share_the_configured_state_dir(
    reader: PReader, config: Config
) -> None:
    assert reader.states.state_dir == config.state_dir
    assert reader.states.path(TEST_STATE_NAME).parent == config.state_dir
