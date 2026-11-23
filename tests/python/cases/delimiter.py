import pytest

from common.constants import TEST_DEFAULT_DELIMITER

DELIMITERS = [
    pytest.param(TEST_DEFAULT_DELIMITER[0], id="comma"),
    pytest.param(ord("\n"), id="newline"),
    pytest.param(0x00, id="null"),
    pytest.param(ord("x"), id="letter"),
    pytest.param(0x7F, id="last_ascii"),
    pytest.param(0x80, id="first_non_ascii"),
    pytest.param(0xE9, id="high_byte"),
    pytest.param(0xFF, id="last"),
]
