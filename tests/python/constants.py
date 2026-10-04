import string
import sys

TEST_STATE_NAME = "job-1"
TEST_DEFAULT_DELIMITER = ","
TEST_UNSAFE_STATE_NAMES = {
    "../../etc/passwd": "path escapes root",
    "/tmp": "path escapes root",  # noqa: S108
    "": "path must not be empty",
    ".": "path must not be empty",
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
TEST_DIRECTORY_ERRORS = (IsADirectoryError, PermissionError)
TEST_LONG_NAME_ERROR = (
    "syntax is incorrect" if sys.platform == "win32" else "File name too long"
)
TEST_UNRESOLVED_PATH_ERROR = (
    r"cannot find the file|cannot be resolved"
    if sys.platform == "win32"
    else r"No such file|Too many levels"
)
