import pytest

MALFORMED_PAYLOADS = [
    pytest.param("not valid json", "expected ident", id="unparsable"),
    pytest.param("{}", "missing field", id="missing_fields"),
    pytest.param("", "EOF while parsing", id="empty"),
]
MALFORMED_PAYLOAD_TEXTS = [
    pytest.param(case.values[0], id=case.id) for case in MALFORMED_PAYLOADS
]
