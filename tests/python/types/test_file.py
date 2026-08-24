from preader import FileMetadata


def test_file_metadata_fields(reader, tmp_file, fingerprint):
    metadata = reader.bytes(tmp_file).state.file

    assert isinstance(metadata, FileMetadata)
    assert metadata.path == tmp_file
    assert metadata.size == len(tmp_file.read_bytes())
    assert metadata.mtime.timestamp() == int(tmp_file.stat().st_mtime)
    assert metadata.fingerprint == fingerprint(tmp_file)


def test_file_metadata_fingerprint_ignores_bytes_beyond_4096(reader, tmp_large_file, fingerprint):
    assert len(tmp_large_file.read_bytes()) > 4096

    metadata = reader.bytes(tmp_large_file).state.file
    assert metadata.fingerprint == fingerprint(tmp_large_file)


def test_file_metadata_repr(reader, tmp_file, expected_repr):
    metadata = reader.bytes(tmp_file).state.file

    assert repr(metadata) == expected_repr(
        "FileMetadata",
        path=f"'{metadata.path}'",
        size=metadata.size,
        mtime=int(metadata.mtime.timestamp()),
        fingerprint=f"'{metadata.fingerprint}'",
    )
