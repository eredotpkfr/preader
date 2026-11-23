import copy
import pickle

from collections.abc import Callable

import pytest

from cases.exception import STATE_ERROR_ARGUMENTS, STATE_ERROR_BASES, STATE_ERRORS
from common.constants import TEST_STATE_NAME
from preader import StateError

DUPLICATES = [
    pytest.param(lambda error: pickle.loads(pickle.dumps(error)), id="pickle"),  # noqa: S301
    pytest.param(copy.copy, id="copy"),
]


@pytest.mark.parametrize("error", STATE_ERRORS)
@pytest.mark.parametrize(("args", "message", "name"), STATE_ERROR_ARGUMENTS)
def test_state_error_fields(
    error: type[StateError], args: tuple[str, ...], message: str, name: str
) -> None:
    raised = error(*args)

    assert str(raised) == message
    assert raised.name == name
    assert raised.args == args


@pytest.mark.parametrize("error", STATE_ERRORS)
def test_state_error_is_documented(error: type[StateError]) -> None:
    assert error.__doc__


@pytest.mark.parametrize(("error", "base"), STATE_ERROR_BASES)
def test_state_error_extends_its_base(
    error: type[StateError], base: type[Exception]
) -> None:
    assert error.__base__ is base


@pytest.mark.parametrize("error", STATE_ERRORS)
def test_state_error_raises_when_an_argument_is_a_keyword(
    error: type[StateError],
) -> None:
    with pytest.raises(TypeError, match="positional-only"):
        error(message="foo")  # type: ignore[call-arg]


@pytest.mark.parametrize("error", STATE_ERRORS)
@pytest.mark.parametrize("duplicate", DUPLICATES)
def test_state_error_survives_duplication(
    error: type[StateError], duplicate: Callable[[StateError], StateError]
) -> None:
    duplicated = duplicate(error("foo", TEST_STATE_NAME))

    assert type(duplicated) is error
    assert str(duplicated) == "foo"
    assert duplicated.name == TEST_STATE_NAME
