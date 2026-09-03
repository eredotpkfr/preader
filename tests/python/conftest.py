import hashlib
import itertools
import os

from collections.abc import Callable, Iterable, Iterator
from pathlib import Path
from typing import Any

import pytest

from preader import Config, PReader, StateRegistry

FINGERPRINT_BYTES = 4096


@pytest.fixture
def config(tmp_path: Path) -> Config:
    return Config(state_dir=tmp_path / "preader")


@pytest.fixture
def reader(config: Config) -> PReader:
    return PReader(config=config)


@pytest.fixture
def registry(reader: PReader) -> StateRegistry:
    return reader.states


@pytest.fixture
def make_reader(tmp_path: Path) -> Callable[..., PReader]:
    def _make(**kwargs: Any) -> PReader:  # noqa: ANN401
        return PReader(config=Config(state_dir=tmp_path / "preader", **kwargs))

    return _make


@pytest.fixture
def make_file(tmp_path: Path) -> Callable[..., Path]:
    def _make(content: bytes, name: str = "data.bin") -> Path:
        path = tmp_path / name
        path.write_bytes(content)

        return path

    return _make


@pytest.fixture
def append() -> Callable[[Path, bytes], None]:
    def _append(path: Path, content: bytes) -> None:
        with path.open("ab") as f:
            f.write(content)

    return _append


@pytest.fixture
def tmp_file(make_file: Callable[..., Path]) -> Path:
    return make_file(b"foo\n")


@pytest.fixture
def tmp_large_file(make_file: Callable[..., Path]) -> Path:
    return make_file(b"foo\n" * 2560, name="large.bin")  # 10 KiB


@pytest.fixture
def empty_file(make_file: Callable[..., Path]) -> Path:
    return make_file(b"", name="empty.bin")


@pytest.fixture
def reindent() -> Callable[[str, int], str]:
    def _reindent(text: str, spaces: int) -> str:
        return text.replace("\n", "\n" + " " * spaces)

    return _reindent


@pytest.fixture
def expected_repr() -> Callable[..., str]:
    def _expected_repr(class_name: str, /, **fields: object) -> str:
        body = ",\n".join(f"  {key}={value}" for key, value in fields.items())
        return f"{class_name}(\n{body}\n)"

    return _expected_repr


@pytest.fixture
def consume() -> Callable[..., Any]:
    def _consume[AnyIterator: Iterable[Any]](
        iterator: AnyIterator, count: int | None = None
    ) -> AnyIterator:
        for _ in itertools.islice(iterator, count):
            pass

        return iterator

    return _consume


@pytest.fixture
def fingerprint() -> Callable[..., str]:
    def _fingerprint(path: Path, window: int = FINGERPRINT_BYTES) -> str:
        return hashlib.sha256(path.read_bytes()[:window]).hexdigest()

    return _fingerprint


@pytest.fixture
def requires_symlinks(tmp_path: Path) -> None:
    try:
        (tmp_path / "link").symlink_to(tmp_path / "target")
    except (NotImplementedError, OSError):
        pytest.skip("creating a symlink is not permitted here")


@pytest.fixture
def requires_non_utf8_names(tmp_path: Path) -> None:
    try:
        probe = tmp_path / os.fsdecode(b"probe-\xff.bin")
        probe.write_bytes(b"")
    except (OSError, UnicodeError):
        pytest.skip("a non-UTF-8 file name cannot be created here")

    probe.unlink()


@pytest.fixture
def revoke_permissions(tmp_path: Path) -> Iterator[Callable[[Path], Path]]:
    probe = tmp_path / "probe"

    probe.mkdir()
    probe.chmod(0o000)

    enforced = not os.access(probe, os.R_OK)

    probe.chmod(0o755)

    if not enforced:
        pytest.skip("file permissions are not enforced here")

    blocked: list[Path] = []

    def _revoke_permissions(path: Path) -> Path:
        blocked.append(path)
        path.chmod(0o000)

        return path

    yield _revoke_permissions

    for path in blocked:
        path.chmod(0o755 if path.is_dir() else 0o644)


@pytest.fixture
def requires_pre_epoch_mtime(tmp_path: Path) -> None:
    probe = tmp_path / "probe.bin"
    probe.write_bytes(b"")

    try:
        os.utime(probe, (-86400, -86400))
    except (OSError, OverflowError):
        pytest.skip("a pre-epoch mtime cannot be set here")
