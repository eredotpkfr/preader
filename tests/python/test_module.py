import preader
import pytest

EXPORTS = (
    "ByteIterator",
    "ChunkIterator",
    "Config",
    "DelimiterIterator",
    "FileMetadata",
    "IteratorBase",
    "IteratorOptions",
    "LineIterator",
    "PReader",
    "State",
    "StateError",
    "StateIterator",
    "StateRegistry",
    "Timestamps",
)


@pytest.mark.parametrize("name", EXPORTS)
def test_module_exports_the_public_api(name):
    assert isinstance(getattr(preader, name), type)


def test_module_exports_nothing_unexpected():
    exported = {
        name for name, value in vars(preader).items() if isinstance(value, type)
    }

    assert exported == set(EXPORTS)
