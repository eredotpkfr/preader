from collections.abc import Callable
from contextlib import AbstractContextManager
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import pytest

from common.constants import TEST_STATE_NAME
from preader import PReader, Timestamps


def test_timestamps_fields(reader: PReader, tmp_file: Path) -> None:
    before = datetime.now(UTC).replace(microsecond=0)
    timestamps = reader.bytes(tmp_file).state.timestamps
    after = datetime.now(UTC)

    assert isinstance(timestamps, Timestamps)
    assert timestamps.created_at == timestamps.updated_at
    assert timestamps.created_at.microsecond == 0
    assert timestamps.created_at.tzinfo is UTC
    assert timestamps.updated_at.tzinfo is UTC
    assert before <= timestamps.created_at <= after


def test_timestamps_compares_by_value(
    make_reader: Callable[..., PReader],
    tmp_file: Path,
    tamper: Callable[[Path], AbstractContextManager[dict[str, Any]]],
) -> None:
    reader = make_reader(verify_state=False)
    saved = reader.bytes(tmp_file, state=TEST_STATE_NAME).save()

    with tamper(reader.states.path(TEST_STATE_NAME)) as payload:
        payload["timestamps"]["created_at"] = 0

    assert saved.timestamps is not saved.timestamps
    assert saved.timestamps == saved.timestamps
    assert reader.states[TEST_STATE_NAME].timestamps != saved.timestamps


def test_timestamps_repr(
    reader: PReader, tmp_file: Path, expected_repr: Callable[..., str]
) -> None:
    timestamps = reader.bytes(tmp_file).state.timestamps

    assert repr(timestamps) == expected_repr(
        "Timestamps", created_at=timestamps.created_at, updated_at=timestamps.updated_at
    )


@pytest.mark.parametrize("field", ["created_at", "updated_at"])
def test_timestamps_raises_when_a_field_is_set(
    reader: PReader, tmp_file: Path, field: str
) -> None:
    timestamps = reader.bytes(tmp_file).state.timestamps

    with pytest.raises(AttributeError, match="is not writable"):
        setattr(timestamps, field, None)
