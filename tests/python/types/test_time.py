from collections.abc import Callable
from pathlib import Path

from preader import PReader, Timestamps


def test_timestamps_fields(reader: PReader, tmp_file: Path) -> None:
    timestamps = reader.bytes(tmp_file).state.timestamps

    assert isinstance(timestamps, Timestamps)
    assert timestamps.created_at == timestamps.updated_at


def test_timestamps_compares_by_value(reader: PReader, tmp_file: Path) -> None:
    state = reader.bytes(tmp_file).state

    assert state.timestamps is not state.timestamps
    assert state.timestamps == state.timestamps
    assert state.timestamps != state.file


def test_timestamps_repr(
    reader: PReader, tmp_file: Path, expected_repr: Callable[..., str]
) -> None:
    timestamps = reader.bytes(tmp_file).state.timestamps

    assert repr(timestamps) == expected_repr(
        "Timestamps",
        created_at=int(timestamps.created_at.timestamp()),
        updated_at=int(timestamps.updated_at.timestamp()),
    )
