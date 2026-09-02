import hashlib
import itertools
import os

import pytest
from preader import Config, PReader

FINGERPRINT_BYTES = 4096


@pytest.fixture
def config(tmp_path):
    return Config(state_dir=tmp_path / "preader")


@pytest.fixture
def reader(config):
    return PReader(config=config)


@pytest.fixture
def registry(reader):
    return reader.states


@pytest.fixture
def make_reader(tmp_path):
    def _make(**kwargs):
        return PReader(config=Config(state_dir=tmp_path / "preader", **kwargs))

    return _make


@pytest.fixture
def make_file(tmp_path):
    def _make(content: bytes, name: str = "data.bin"):
        path = tmp_path / name
        path.write_bytes(content)

        return path

    return _make


@pytest.fixture
def append():
    def _append(path, content):
        with open(path, "ab") as f:
            f.write(content)

    return _append


@pytest.fixture
def tmp_file(make_file):
    return make_file(b"foo\n")


@pytest.fixture
def tmp_large_file(make_file):
    return make_file(b"foo\n" * 2560)  # 10 KiB


@pytest.fixture
def empty_file(make_file):
    return make_file(b"")


@pytest.fixture
def reindent():
    def _reindent(text, spaces):
        return text.replace("\n", "\n" + " " * spaces)

    return _reindent


@pytest.fixture
def expected_repr():
    def _expected_repr(class_name, /, **fields):
        body = ",\n".join(f"  {key}={value}" for key, value in fields.items())
        return f"{class_name}(\n{body}\n)"

    return _expected_repr


@pytest.fixture
def consume():
    def _consume(iterator, count=None):
        for _ in itertools.islice(iterator, count):
            pass

        return iterator

    return _consume


@pytest.fixture
def fingerprint():
    def _fingerprint(path, window=FINGERPRINT_BYTES):
        return hashlib.sha256(path.read_bytes()[:window]).hexdigest()

    return _fingerprint


@pytest.fixture
def requires_symlinks(tmp_path):
    try:
        (tmp_path / "link").symlink_to(tmp_path / "target")
    except (NotImplementedError, OSError):
        pytest.skip("creating a symlink is not permitted here")


@pytest.fixture
def requires_non_utf8_names(tmp_path):
    probe = tmp_path / os.fsdecode(b"probe-\xff.bin")

    try:
        probe.write_bytes(b"")
    except (OSError, UnicodeError):
        pytest.skip("a non-UTF-8 file name cannot be created here")

    probe.unlink()


@pytest.fixture
def revoke_permissions(tmp_path):
    probe = tmp_path / "probe"

    probe.mkdir()
    probe.chmod(0o000)

    enforced = not os.access(probe, os.R_OK)

    probe.chmod(0o755)

    if not enforced:
        pytest.skip("file permissions are not enforced here")

    blocked = []

    def _revoke_permissions(path):
        blocked.append(path)
        path.chmod(0o000)

        return path

    yield _revoke_permissions

    for path in blocked:
        path.chmod(0o755 if path.is_dir() else 0o644)


@pytest.fixture
def requires_pre_epoch_mtime(tmp_path):
    probe = tmp_path / "probe.bin"
    probe.write_bytes(b"")

    try:
        os.utime(probe, (-86400, -86400))
    except (OSError, OverflowError):
        pytest.skip("a pre-epoch mtime cannot be set here")
