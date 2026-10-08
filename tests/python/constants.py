import os
import re
import string
import sys

TEST_STATE_NAME = "job-1"
TEST_DEFAULT_DELIMITER = ","
TEST_UNSAFE_STATE_MESSAGES = {
    "../../etc/passwd": "path escapes root: ../../etc/passwd",
    "..": "path escapes root: ..",
    "foo/../bar": "path escapes root: foo/../bar",
    "/": "path escapes root: /",
    "/tmp": "path escapes root: /tmp",  # noqa: S108
    "": "path must not be empty",
    ".": "path must not be empty",
}
TEST_WINDOWS_UNSAFE_STATE_MESSAGES = {
    r"C:\job-1": r"path escapes root: C:\job-1",
    "C:/job-1": r"path escapes root: C:\job-1",
    "C:job-1": "path escapes root: C:job-1",
    r"\job-1": r"path escapes root: \job-1",
    r"\\server\share\job-1": r"path escapes root: \\server\share\job-1",
    r"\\?\C:\job-1": r"path escapes root: \\?\C:\job-1",
    r"..\..\etc\passwd": r"path escapes root: ..\..\etc\passwd",
}
TEST_UNSAFE_STATE_NAMES = {
    name: f"^{re.escape(message.replace('/', os.sep))}$"
    for name, message in TEST_UNSAFE_STATE_MESSAGES.items()
}
TEST_UNSAFE_STATE_NAME_IDS = (
    "traversal",
    "parent",
    "inner_traversal",
    "root",
    "absolute",
    "empty",
    "current_dir",
)
TEST_WINDOWS_UNSAFE_STATE_NAMES = {
    name: f"^{re.escape(message)}$"
    for name, message in TEST_WINDOWS_UNSAFE_STATE_MESSAGES.items()
}
TEST_WINDOWS_UNSAFE_STATE_NAME_IDS = (
    "drive_absolute",
    "drive_forward_slash",
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
