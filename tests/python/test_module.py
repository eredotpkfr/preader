import pytest

import preader

from cases.export import EXPORTS, FINAL, SUBCLASSABLE


@pytest.mark.parametrize("name", SUBCLASSABLE)
def test_subclassable_export_accepts_subclasses(name: str) -> None:
    base = getattr(preader, name)

    assert issubclass(type(f"Sub{name}", (base,), {}), base)


@pytest.mark.parametrize("name", FINAL)
def test_final_export_raises_when_it_is_subclassed(name: str) -> None:
    base = getattr(preader, name)

    with pytest.raises(TypeError, match="not an acceptable base type"):
        type(f"Sub{name}", (base,), {})


def test_module_exports_nothing_unexpected() -> None:
    exported = {
        name for name, value in vars(preader).items() if isinstance(value, type)
    }

    assert exported == set(EXPORTS)
