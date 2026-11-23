import hashlib
import itertools
import json
import os
import random

from collections.abc import Callable, Generator, Iterable, Iterator
from contextlib import AbstractContextManager, contextmanager
from datetime import UTC, datetime, timedelta
from pathlib import Path
from typing import Any

import hypothesis
import pytest

from common.constants import (
    TEST_CASES,
    TEST_LARGE_COPIES,
    TEST_LINE,
    TEST_PRE_EPOCH,
    TEST_WINDOW,
)
from preader import Config, PReader, State, StateRegistry

pytest.register_assert_rewrite("common.matrix")
hypothesis.settings.register_profile(
    "preader",
    database=None,
    deadline=None,
    max_examples=TEST_CASES,
    suppress_health_check=[hypothesis.HealthCheck.function_scoped_fixture],
)
hypothesis.settings.load_profile("preader")


@pytest.hookimpl(tryfirst=True)
def pytest_runtest_call(item: pytest.Item) -> None:
    run_seed = item.config.getoption("randomly_seed", None)

    if (
        run_seed is not None
        and isinstance(item, pytest.Function)
        and hypothesis.is_hypothesis_test(item.obj)
    ):
        hypothesis.seed(f"{run_seed}:{item.nodeid}")(item.obj)


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
def make_reader(config: Config) -> Callable[..., PReader]:
    def _make(**kwargs: Any) -> PReader:  # noqa: ANN401
        kwargs.setdefault("state_dir", config.state_dir)

        return PReader(config=Config(**kwargs))

    return _make


@pytest.fixture
def make_file(tmp_path: Path) -> Callable[..., Path]:
    def _make(content: bytes, name: str = "data.bin") -> Path:
        path = tmp_path / name
        path.write_bytes(content)

        return path

    return _make


@pytest.fixture
def make_stray(config: Config) -> Callable[[str], Path]:
    def _make_stray(name: str) -> Path:
        stray = config.state_dir / name
        stray.parent.mkdir(parents=True, exist_ok=True)
        stray.write_text("{}", encoding="utf-8")

        return stray

    return _make_stray


@pytest.fixture
def append() -> Callable[[Path, bytes], None]:
    def _append(path: Path, content: bytes) -> None:
        with path.open("ab") as f:
            f.write(content)

    return _append


@pytest.fixture
def tamper() -> Callable[[Path], AbstractContextManager[dict[str, Any]]]:
    @contextmanager
    def _tamper(path: Path) -> Generator[dict[str, Any]]:
        payload: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))

        yield payload

        path.write_text(json.dumps(payload), encoding="utf-8")

    return _tamper


@pytest.fixture
def truncate() -> Callable[[Path, int], None]:
    def _truncate(path: Path, size: int) -> None:
        with path.open("r+b") as f:
            f.truncate(size)

    return _truncate


@pytest.fixture
def tmp_file(make_file: Callable[..., Path]) -> Path:
    return make_file(TEST_LINE)


@pytest.fixture
def tmp_large_file(make_file: Callable[..., Path]) -> Path:
    return make_file(TEST_LINE * TEST_LARGE_COPIES, name="large.bin")  # 10 KiB


@pytest.fixture
def empty_file(make_file: Callable[..., Path]) -> Path:
    return make_file(b"", name="empty.bin")


@pytest.fixture
def seed() -> int:
    return random.getrandbits(64)


@pytest.fixture
def expected_repr() -> Callable[..., str]:
    def _expected_repr(class_name: str, /, **fields: object) -> str:
        body = ", ".join(f"{key}={value!r}" for key, value in fields.items())

        return f"{class_name}({body})"

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
    def _fingerprint(path: Path) -> str:
        return hashlib.sha256(path.read_bytes()[:TEST_WINDOW]).hexdigest()

    return _fingerprint


@pytest.fixture
def stat_mtime() -> Callable[[Path], datetime]:
    def _stat_mtime(path: Path) -> datetime:
        micros = path.stat().st_mtime_ns // 1_000

        return datetime.fromtimestamp(0, UTC) + timedelta(microseconds=micros)

    return _stat_mtime


@pytest.fixture(scope="session")
def requires_symlinks(tmp_path_factory: pytest.TempPathFactory) -> None:
    probe = tmp_path_factory.mktemp("probe")

    try:
        (probe / "link").symlink_to(probe / "target")
    except (NotImplementedError, OSError):
        pytest.skip("creating a symlink is not permitted here")


@pytest.fixture(scope="session")
def requires_non_utf8_names(tmp_path_factory: pytest.TempPathFactory) -> None:
    probe = tmp_path_factory.mktemp("probe")

    try:
        (probe / os.fsdecode(b"probe-\xff.bin")).write_bytes(b"")
    except (OSError, UnicodeError):
        pytest.skip("a non-UTF-8 file name cannot be created here")


@pytest.fixture(scope="session")
def requires_enforced_permissions(tmp_path_factory: pytest.TempPathFactory) -> None:
    probe = tmp_path_factory.mktemp("probe")

    probe.chmod(0o000)

    enforced = not os.access(probe, os.R_OK)

    probe.chmod(0o755)

    if not enforced:
        pytest.skip("file permissions are not enforced here")


@pytest.fixture
def revoke_permissions(
    requires_enforced_permissions: None,  # noqa: ARG001
) -> Iterator[Callable[..., Path]]:
    blocked: list[Path] = []

    def _revoke_permissions(path: Path, mode: int = 0o000) -> Path:
        blocked.append(path)
        path.chmod(mode)

        return path

    yield _revoke_permissions

    for path in blocked:
        path.chmod(0o755 if path.is_dir() else 0o644)


@pytest.fixture(scope="session")
def requires_pre_epoch_mtime(tmp_path_factory: pytest.TempPathFactory) -> None:
    probe = tmp_path_factory.mktemp("probe") / "probe.bin"
    probe.write_bytes(b"")

    try:
        os.utime(probe, (TEST_PRE_EPOCH, TEST_PRE_EPOCH))
    except (OSError, OverflowError):
        pytest.skip("a pre-epoch mtime cannot be set here")


@pytest.fixture
def read_state() -> Callable[[Path, State], dict[str, Any]]:
    def _read_state(path: Path, state: State) -> dict[str, Any]:
        payload: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
        body = {key: value for key, value in payload.items() if key != "_checksum"}
        digest = hashlib.sha256(
            json.dumps(body, separators=(",", ":"), ensure_ascii=False).encode()
        )
        metadata, stamps = state.file, state.timestamps

        assert body == {
            "name": state.name,
            "file": {
                "path": str(metadata.path),
                "size": metadata.size,
                "mtime": (metadata.mtime - datetime.fromtimestamp(0, UTC))
                // timedelta(microseconds=1),
                "fingerprint": metadata.fingerprint,
            },
            "position": state.position,
            "timestamps": {
                "created_at": int(stamps.created_at.timestamp()),
                "updated_at": int(stamps.updated_at.timestamp()),
            },
        }
        assert payload["_checksum"] == digest.hexdigest()

        return payload

    return _read_state
