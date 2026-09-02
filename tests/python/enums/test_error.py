import pytest
from constants import TEST_STATE_NAME
from preader import StateError


def test_io_errors_get_prefixed(reader, tmp_file):
    state = reader.bytes(tmp_file, state=TEST_STATE_NAME).state

    state.save()
    tmp_file.unlink()

    with pytest.raises(StateError, match=r"^io failed \(NotFound\): "):
        state.verify()


def test_serde_errors_get_prefixed(registry):
    path = registry.path(TEST_STATE_NAME)

    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text("not valid json")

    with pytest.raises(StateError, match=r"^serde failed: "):
        registry[TEST_STATE_NAME]


def test_regex_errors_get_prefixed(registry):
    with pytest.raises(StateError, match=r"^regex failed: "):
        registry.search("[invalid(")


def test_anyhow_errors_are_not_prefixed(registry):
    with pytest.raises(StateError) as exc_info:
        registry.path("../../etc/passwd")

    message = str(exc_info.value)

    assert message == "path escapes root: ../../etc/passwd"
    assert not message.startswith(("io failed", "regex failed", "serde failed"))
