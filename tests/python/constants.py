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
TEST_WINDOWS_UNSAFE_STATE_NAMES = (
    "C:\\job-1",
    "C:job-1",
    "\\job-1",
    "\\\\server\\share\\job-1",
    "\\\\?\\C:\\job-1",
    "..\\..\\etc\\passwd",
)
TEST_WINDOWS_UNSAFE_STATE_NAME_IDS = (
    "drive_absolute",
    "drive_relative",
    "root_relative",
    "unc_share",
    "verbatim_drive",
    "backslash_traversal",
)
TEST_ALPHABET = string.ascii_lowercase.encode()
