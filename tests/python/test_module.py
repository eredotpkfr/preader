import pytest

import preader

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

SUBCLASSABLE = ("IteratorBase", "StateError")
FINAL = tuple(name for name in EXPORTS if name not in SUBCLASSABLE)


@pytest.mark.parametrize("name", EXPORTS)
def test_module_exports_the_public_api(name: str) -> None:
    assert isinstance(getattr(preader, name), type)


def test_module_exports_nothing_unexpected() -> None:
    exported = {
        name for name, value in vars(preader).items() if isinstance(value, type)
    }

    assert exported == set(EXPORTS)


@pytest.mark.parametrize("name", SUBCLASSABLE)
def test_subclassable_export_accepts_subclasses(name: str) -> None:
    base = getattr(preader, name)

    assert issubclass(type(f"Sub{name}", (base,), {}), base)


@pytest.mark.parametrize("name", FINAL)
def test_final_export_raises_when_subclassed(name: str) -> None:
    base = getattr(preader, name)

    with pytest.raises(TypeError, match="not an acceptable base type"):
        type(f"Sub{name}", (base,), {})
