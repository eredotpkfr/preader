import os

from collections.abc import Callable
from datetime import UTC, datetime
from pathlib import Path

import pytest

from common.constants import TEST_PRE_EPOCH, TEST_WINDOW
from preader import FileMetadata, PReader


def test_file_metadata_fields(
    reader: PReader,
    tmp_file: Path,
    fingerprint: Callable[..., str],
    stat_mtime: Callable[[Path], datetime],
) -> None:
    metadata = reader.bytes(tmp_file).state.file

    assert isinstance(metadata, FileMetadata)
    assert metadata.path == tmp_file
    assert metadata.size == len(tmp_file.read_bytes())
    assert metadata.mtime == stat_mtime(tmp_file)
    assert metadata.mtime.tzinfo is UTC
    assert metadata.fingerprint == fingerprint(tmp_file)


def test_file_metadata_compares_by_value(
    reader: PReader, tmp_file: Path, tmp_large_file: Path
) -> None:
    metadata = reader.bytes(tmp_file).state.file

    assert metadata is not reader.bytes(tmp_file).state.file
    assert metadata == reader.bytes(tmp_file).state.file
    assert metadata != reader.bytes(tmp_large_file).state.file


def test_file_metadata_repr(
    reader: PReader, tmp_file: Path, expected_repr: Callable[..., str]
) -> None:
    metadata = reader.bytes(tmp_file).state.file

    assert repr(metadata) == expected_repr(
        "FileMetadata",
        path=metadata.path,
        size=metadata.size,
        mtime=metadata.mtime,
        fingerprint=metadata.fingerprint,
    )


def test_file_metadata_raises_when_the_file_is_unreadable(
    reader: PReader, tmp_file: Path, revoke_permissions: Callable[..., Path]
) -> None:
    revoke_permissions(tmp_file)

    with pytest.raises(PermissionError):
        reader.bytes(tmp_file)


def test_file_metadata_raises_when_the_path_is_a_directory(
    reader: PReader, tmp_path: Path
) -> None:
    directory = tmp_path / "elsewhere"
    directory.mkdir()

    with pytest.raises(IsADirectoryError, match="is a directory"):
        reader.bytes(directory)


def test_file_metadata_fingerprints_an_empty_file(
    reader: PReader, empty_file: Path, fingerprint: Callable[..., str]
) -> None:
    metadata = reader.bytes(empty_file).state.file

    assert metadata.size == 0
    assert metadata.fingerprint == fingerprint(empty_file)


@pytest.mark.parametrize("field", ["fingerprint", "mtime", "path", "size"])
def test_file_metadata_raises_when_a_field_is_set(
    reader: PReader, tmp_file: Path, field: str
) -> None:
    metadata = reader.bytes(tmp_file).state.file

    with pytest.raises(AttributeError, match="is not writable"):
        setattr(metadata, field, None)


@pytest.mark.skipif(not hasattr(os, "mkfifo"), reason="a fifo cannot be created here")
def test_file_metadata_raises_when_the_path_is_a_fifo(
    reader: PReader, tmp_path: Path
) -> None:
    fifo = tmp_path / "a-fifo"

    os.mkfifo(fifo)

    with pytest.raises(OSError, match="not a regular file"):
        reader.bytes(fifo)


def test_file_metadata_fingerprint_ignores_bytes_past_the_window(
    reader: PReader, tmp_large_file: Path, fingerprint: Callable[..., str]
) -> None:
    metadata = reader.bytes(tmp_large_file).state.file

    assert metadata.size > TEST_WINDOW
    assert metadata.fingerprint == fingerprint(tmp_large_file)


@pytest.mark.usefixtures("requires_pre_epoch_mtime")
def test_file_metadata_records_an_mtime_before_the_epoch(
    reader: PReader, tmp_file: Path
) -> None:
    os.utime(tmp_file, (TEST_PRE_EPOCH, TEST_PRE_EPOCH))

    iterator = reader.bytes(tmp_file)

    assert iterator.state.file.mtime.timestamp() == TEST_PRE_EPOCH
    assert b"".join(iterator) == tmp_file.read_bytes()
