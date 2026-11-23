import functools

from collections.abc import Callable
from typing import Any

import pytest

from common.constants import TEST_CHUNK_SIZE
from preader import PReader

_SIZES = (512, 4096, 51_200)
_KINDS = ("random", "text", "zip", "pdf")
_REPRODUCERS: dict[str, Callable[..., Any]] = {
    "bytes": PReader.bytes,
    "chunks": functools.partial(PReader.chunks, chunk_size=TEST_CHUNK_SIZE),
    "segments": functools.partial(PReader.segments, keep_delimiter=True),
    "lines": functools.partial(PReader.lines, keepends=True),
}


CONTENTS = [
    pytest.param(kind, size, id=f"{kind}_{size}b") for kind in _KINDS for size in _SIZES
]
TEXTS = [pytest.param("text", size, id=f"text_{size}b") for size in _SIZES]
ARCHIVES = [pytest.param("zip", size, id=f"zip_{size}b") for size in _SIZES]


def _flows(*names: str) -> list[Any]:
    return [
        pytest.param(_REPRODUCERS[name], kind, size, id=f"{name}_{kind}_{size}b")
        for name in names
        for kind in _KINDS
        for size in _SIZES
        if name != "lines" or kind == "text"
    ]


FLOWS = _flows(*_REPRODUCERS)
WINDOWS = _flows("bytes", "chunks")
