import pytest

from common.constants import TEST_STATE_NAME
from preader import (
    StateConflictError,
    StateCorruptError,
    StateError,
    StateMismatchError,
    StateOutdatedError,
)

STATE_ERRORS = [
    pytest.param(StateError, id="state"),
    pytest.param(StateMismatchError, id="mismatch"),
    pytest.param(StateOutdatedError, id="outdated"),
    pytest.param(StateCorruptError, id="corrupt"),
    pytest.param(StateConflictError, id="conflict"),
]
STATE_ERROR_BASES = [
    pytest.param(StateError, Exception, id="state"),
    pytest.param(StateMismatchError, StateError, id="mismatch"),
    pytest.param(StateOutdatedError, StateMismatchError, id="outdated"),
    pytest.param(StateCorruptError, StateError, id="corrupt"),
    pytest.param(StateConflictError, StateError, id="conflict"),
]
STATE_ERROR_ARGUMENTS = [
    pytest.param((), "", "", id="no_arguments"),
    pytest.param(("foo",), "foo", "", id="message"),
    pytest.param(
        ("foo", TEST_STATE_NAME), "foo", TEST_STATE_NAME, id="message_and_name"
    ),
]
