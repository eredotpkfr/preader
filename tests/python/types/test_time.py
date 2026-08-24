from preader import Timestamps


def test_timestamps_fields(reader, tmp_file):
    timestamps = reader.bytes(tmp_file).state.timestamps

    assert isinstance(timestamps, Timestamps)
    assert timestamps.created_at == timestamps.updated_at


def test_timestamps_repr(reader, tmp_file, expected_repr):
    timestamps = reader.bytes(tmp_file).state.timestamps

    assert repr(timestamps) == expected_repr(
        "Timestamps",
        created_at=int(timestamps.created_at.timestamp()),
        updated_at=int(timestamps.updated_at.timestamp()),
    )
