import string

TEST_STATE_NAME = "job-1"
TEST_DEFAULT_DELIMITER = ","
TEST_UNSAFE_STATE_NAMES = {
    "../../etc/passwd": "path escapes root",
    "/tmp": "path escapes root",
    "": "path must not be empty",
    ".": "path must name an entry",
}
TEST_UNSAFE_STATE_NAME_IDS = ("traversal", "absolute", "empty", "current_dir")
TEST_ALPHABET = string.ascii_lowercase.encode()
