import pytest

from preader import PReader

ITERATORS = [
    pytest.param(PReader.bytes, id="bytes"),
    pytest.param(PReader.chunks, id="chunks"),
    pytest.param(PReader.lines, id="lines"),
    pytest.param(PReader.segments, id="segments"),
]
