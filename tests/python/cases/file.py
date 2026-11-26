import pytest

NON_STATE_FILES = [
    pytest.param("README.md", id="plain"),
    pytest.param("job-1.state.json.bak", id="suffix_shaped"),
    pytest.param("job-1.state", id="partial_suffix"),
    pytest.param("job-1.state.json.tmp", id="temporary_suffix"),
    pytest.param("job-2.STATE.JSON", id="uppercase_suffix"),
    pytest.param("mystate.json", id="only_looks_suffixed"),
]
FOREIGN_STATE_FILES = [
    pytest.param(
        "job-1.state.json.state.json", "job-1.state.json", id="doubled_suffix"
    ),
    pytest.param(
        "sub/job-1.state.json.state.json",
        "sub/job-1.state.json",
        id="nested_doubled_suffix",
    ),
    pytest.param(
        "archive.state.json/job-1.state.json",
        "archive.state.json/job-1",
        id="state_shaped_directory",
    ),
    pytest.param(".job-2.state.json", ".job-2", id="hidden"),
    pytest.param("Job-2.state.json", "Job-2", id="uppercase"),
    pytest.param("café.state.json", "café", id="unicode"),
    pytest.param("job 2.state.json", "job 2", id="space"),
]
UNADDRESSABLE_FILES = [
    pytest.param(".state.json", id="bare_suffix"),
    pytest.param("sub/.state.json", id="nested_bare_suffix"),
    pytest.param("..state.json", id="current_dir_stem"),
    pytest.param("...state.json", id="parent_dir_stem"),
]
UNREPRESENTABLE_FILES = [
    pytest.param("sub\\job-2.state.json", id="backslash"),
    pytest.param("job:2.state.json", id="colon"),
    pytest.param("job*2.state.json", id="asterisk"),
    pytest.param("sub./job-2.state.json", id="directory_ending_in_a_dot"),
    pytest.param("sub /job-2.state.json", id="directory_ending_in_a_space"),
    pytest.param("job\t2.state.json", id="control_character"),
    pytest.param("job\u200b2.state.json", id="format_character"),
    pytest.param("con.state.json", id="device"),
    pytest.param("sub/aux.state.json", id="nested_device"),
    pytest.param("f" * 220 + ".state.json", id="too_long_file"),
    pytest.param(
        "/".join(["d" * 100] * 4) + "/" + "f" * 109 + ".state.json",
        id="too_long_whole_name",
    ),
]
