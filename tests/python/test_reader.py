import os

from collections.abc import Callable
from pathlib import Path
from typing import Any

import pytest

from constants import TEST_ALPHABET, TEST_DEFAULT_DELIMITER, TEST_STATE_NAME
from preader import Config, IteratorOptions, PReader, StateError


def test_preader_defaults() -> None:
    config = PReader().config
    default = Config()

    assert config.buffer_capacity == default.buffer_capacity
    assert config.state_dir == default.state_dir
    assert config.auto_save_state == default.auto_save_state
    assert config.auto_save_state_bytes == default.auto_save_state_bytes
    assert config.auto_load_state == default.auto_load_state
    assert config.verify_state == default.verify_state


def test_raises_when_config_is_none() -> None:
    with pytest.raises(TypeError):
        PReader(config=None)  # type: ignore[arg-type]


def test_buffer_capacity_zero(
    make_reader: Callable[..., PReader], tmp_file: Path
) -> None:
    reader = make_reader(buffer_capacity=0)

    assert b"".join(reader.bytes(tmp_file)) == tmp_file.read_bytes()


def test_bytes_raises_when_file_missing(reader: PReader, tmp_path: Path) -> None:
    with pytest.raises(FileNotFoundError):
        reader.bytes(tmp_path / "does-not-exist.bin")


def test_preader_repr(
    config: Config,
    reader: PReader,
    reindent: Callable[[str, int], str],
    expected_repr: Callable[..., str],
) -> None:
    assert repr(reader) == expected_repr("PReader", config=reindent(repr(config), 2))


@pytest.mark.usefixtures("requires_symlinks")
def test_bytes_follows_symlink(reader: PReader, tmp_file: Path) -> None:
    symlink = tmp_file.parent / "link.bin"
    symlink.symlink_to(tmp_file)

    state = reader.bytes(symlink).state

    assert state.file.path == tmp_file


def test_bytes_raises_when_path_is_a_directory(reader: PReader, tmp_path: Path) -> None:
    directory = tmp_path / "a-directory"
    directory.mkdir()

    with pytest.raises(StateError, match="not a file"):
        reader.bytes(directory)


def test_bytes_raises_when_the_file_is_unreadable(
    reader: PReader, tmp_file: Path, revoke_permissions: Callable[[Path], Path]
) -> None:
    revoke_permissions(tmp_file)

    with pytest.raises(StateError, match="Permission denied"):
        reader.bytes(tmp_file)


@pytest.mark.usefixtures("requires_non_utf8_names")
def test_bytes_raises_when_path_is_not_utf8(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo", name=os.fsdecode(b"data-\xff.bin"))

    with pytest.raises(StateError, match="invalid UTF-8"):
        reader.bytes(path)


@pytest.mark.usefixtures("requires_pre_epoch_mtime")
def test_bytes_raises_when_mtime_precedes_the_epoch(
    reader: PReader, tmp_file: Path
) -> None:
    os.utime(tmp_file, (-86400, -86400))

    with pytest.raises(StateError, match="second time provided"):
        reader.bytes(tmp_file)


@pytest.mark.skipif(not hasattr(os, "mkfifo"), reason="a fifo cannot be created here")
def test_bytes_raises_when_path_is_a_fifo(reader: PReader, tmp_path: Path) -> None:
    fifo = tmp_path / "a-fifo"

    os.mkfifo(fifo)

    with pytest.raises(StateError, match="not a file"):
        reader.bytes(fifo)


def test_bytes_yields_nothing_for_empty_file(reader: PReader, empty_file: Path) -> None:
    assert list(reader.bytes(empty_file)) == []


def test_bytes_yields_one_item_for_a_single_byte_file(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    assert list(reader.bytes(make_file(b"a"))) == [b"a"]


@pytest.mark.parametrize(
    "iterate",
    [
        lambda reader, path, options: reader.bytes(path, options=options),
        lambda reader, path, options: reader.chunks(path, options=options),
        lambda reader, path, options: reader.lines(path, options=options),
        lambda reader, path, options: reader.delimiter(
            path, options=options, delimiter=TEST_DEFAULT_DELIMITER
        ),
    ],
    ids=["bytes", "chunks", "lines", "delimiter"],
)
def test_raises_when_start_exceeds_end(
    reader: PReader, tmp_file: Path, iterate: Callable[..., Any]
) -> None:
    with pytest.raises(ValueError, match=r"start .* must be <= end"):
        iterate(reader, tmp_file, IteratorOptions(start=10, end=5))


def test_bytes_treats_an_explicit_none_state_as_auto(
    reader: PReader, tmp_file: Path
) -> None:
    assert (
        reader.bytes(tmp_file, state=None).state.name
        == reader.bytes(tmp_file).state.name
    )


def test_chunks_splits_into_fixed_size_pieces(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(TEST_ALPHABET)
    chunk_size = 3

    chunks = list(reader.chunks(path, chunk_size=chunk_size))
    expected = [
        TEST_ALPHABET[i : i + chunk_size]
        for i in range(0, len(TEST_ALPHABET), chunk_size)
    ]

    assert chunks == expected


def test_chunks_drop_partial_discards_partial_chunk(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(TEST_ALPHABET)
    chunk_size = 3

    chunks = list(reader.chunks(path, chunk_size=chunk_size, drop_partial=True))

    full_length = (len(TEST_ALPHABET) // chunk_size) * chunk_size
    expected = [
        TEST_ALPHABET[i : i + chunk_size] for i in range(0, full_length, chunk_size)
    ]

    assert chunks == expected


def test_chunks_drop_partial_keeps_exact_chunk(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"abcdef")

    assert list(reader.chunks(path, chunk_size=3, drop_partial=True)) == [
        b"abc",
        b"def",
    ]


def test_chunks_single_chunk_for_large_chunk_size(
    reader: PReader, tmp_file: Path
) -> None:
    assert list(reader.chunks(tmp_file, chunk_size=1024)) == [tmp_file.read_bytes()]


def test_chunks_drop_partial_empty_for_large_chunk_size(
    reader: PReader, tmp_file: Path
) -> None:
    assert list(reader.chunks(tmp_file, chunk_size=1024, drop_partial=True)) == []


@pytest.mark.parametrize(
    "drop_partial", [True, False], ids=["drop_partial", "defaults"]
)
def test_chunks_yields_nothing_for_empty_file(
    reader: PReader, empty_file: Path, drop_partial: bool
) -> None:
    assert list(reader.chunks(empty_file, drop_partial=drop_partial)) == []


@pytest.mark.parametrize(
    ("drop_partial", "expected"),
    [(True, []), (False, [b"a"])],
    ids=["drop_partial", "defaults"],
)
def test_chunks_yields_one_item_for_a_single_byte_file(
    reader: PReader,
    make_file: Callable[..., Path],
    drop_partial: bool,
    expected: object,
) -> None:
    assert list(reader.chunks(make_file(b"a"), drop_partial=drop_partial)) == expected


@pytest.mark.parametrize(
    "drop_partial", [True, False], ids=["drop_partial", "defaults"]
)
def test_chunks_attributes(reader: PReader, tmp_file: Path, drop_partial: bool) -> None:
    iterator = reader.chunks(tmp_file, chunk_size=1024, drop_partial=drop_partial)

    assert iterator.chunk_size == 1024
    assert iterator.drop_partial is drop_partial


@pytest.mark.parametrize(
    ("content", "expected"),
    [
        (b"foo\nbar\nbaz\n", ["foo", "bar", "baz"]),
        (b"foo\n\nbar\n", ["foo", "", "bar"]),
    ],
    ids=["plain", "with_a_blank_line"],
)
def test_lines_splits_on_newline(
    reader: PReader, make_file: Callable[..., Path], content: bytes, expected: list[str]
) -> None:
    assert list(reader.lines(make_file(content))) == expected


def test_lines_keepends_preserves_newline(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\nbar\n")

    assert list(reader.lines(path, keepends=True)) == ["foo\n", "bar\n"]


def test_lines_keepends_keeps_blank_lines(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\n\nbar\n")

    assert list(reader.lines(path, keepends=True)) == ["foo\n", "\n", "bar\n"]


def test_lines_yields_nothing_for_empty_file(reader: PReader, empty_file: Path) -> None:
    assert list(reader.lines(empty_file)) == []


def test_lines_yields_one_item_for_a_single_byte_file(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    assert list(reader.lines(make_file(b"a"))) == ["a"]


def test_lines_skip_empty_skips_blank_lines(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\n\nbar\n")

    assert list(reader.lines(path, skip_empty=True)) == ["foo", "bar"]


def test_lines_skip_empty_with_keepends(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\n\nbar\n")

    assert list(reader.lines(path, keepends=True, skip_empty=True)) == [
        "foo\n",
        "bar\n",
    ]


@pytest.mark.parametrize(
    ("keepends", "skip_empty"),
    [(True, True), (True, False), (False, True), (False, False)],
    ids=["both", "keepends", "skip_empty", "defaults"],
)
def test_lines_attributes(
    reader: PReader, tmp_file: Path, keepends: bool, skip_empty: bool
) -> None:
    options = IteratorOptions(skip=1)
    iterator = reader.lines(
        tmp_file, options=options, keepends=keepends, skip_empty=skip_empty
    )

    assert iterator.keepends is keepends
    assert iterator.skip_empty is skip_empty
    assert iterator.skip_remaining == options.skip


@pytest.mark.parametrize(
    ("content", "expected"),
    [(b"foo,bar,baz", [b"foo", b"bar", b"baz"]), (b"foo,,bar,", [b"foo", b"", b"bar"])],
    ids=["plain", "with_a_blank_segment"],
)
def test_delimiter_splits_on_byte(
    reader: PReader,
    make_file: Callable[..., Path],
    content: bytes,
    expected: list[bytes],
) -> None:
    assert list(reader.delimiter(make_file(content), delimiter=",")) == expected


def test_delimiter_keep_delimiter_preserves_byte(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo,bar,")

    assert list(reader.delimiter(path, delimiter=",", keep_delimiter=True)) == [
        b"foo,",
        b"bar,",
    ]


def test_delimiter_keep_delimiter_keeps_blank_segments(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo,,bar,")

    segments = list(reader.delimiter(path, delimiter=",", keep_delimiter=True))

    assert segments == [b"foo,", b",", b"bar,"]


def test_delimiter_yields_nothing_for_empty_file(
    reader: PReader, empty_file: Path
) -> None:
    assert list(reader.delimiter(empty_file, delimiter=",")) == []


def test_delimiter_yields_one_item_for_a_single_byte_file(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    assert list(reader.delimiter(make_file(b"a"), delimiter=",")) == [b"a"]


def test_delimiter_skip_empty_skips_blank_segments(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo,,bar,")

    assert list(reader.delimiter(path, delimiter=",", skip_empty=True)) == [
        b"foo",
        b"bar",
    ]


def test_delimiter_skip_empty_with_keep_delimiter(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo,,bar,")

    segments = list(
        reader.delimiter(path, delimiter=",", keep_delimiter=True, skip_empty=True)
    )

    assert segments == [b"foo,", b"bar,"]


@pytest.mark.parametrize(
    ("keep_delimiter", "skip_empty"),
    [(True, True), (True, False), (False, True), (False, False)],
    ids=["both", "keep_delimiter", "skip_empty", "defaults"],
)
def test_delimiter_attributes(
    reader: PReader, tmp_file: Path, keep_delimiter: bool, skip_empty: bool
) -> None:
    options = IteratorOptions(skip=1)
    iterator = reader.delimiter(
        tmp_file,
        delimiter=",",
        options=options,
        keep_delimiter=keep_delimiter,
        skip_empty=skip_empty,
    )

    assert iterator.delimiter == ord(",")
    assert iterator.keep_delimiter is keep_delimiter
    assert iterator.skip_empty is skip_empty
    assert iterator.skip_remaining == options.skip


def test_delimiter_accepts_codepoint_up_to_255(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo" + bytes([ord("é")]) + b"bar")

    assert list(reader.delimiter(path, delimiter="é")) == [b"foo", b"bar"]


def test_delimiter_raises_when_codepoint_above_255(
    reader: PReader, tmp_file: Path
) -> None:
    with pytest.raises(ValueError, match="delimiter must fit in a single byte"):
        reader.delimiter(tmp_file, delimiter="€")


@pytest.mark.parametrize(
    "name",
    ["café-日本語.bin", "foo bar; $baz & `qux`.bin"],
    ids=["unicode", "special_characters"],
)
def test_bytes_reads_an_unusual_path(
    reader: PReader, make_file: Callable[..., Path], name: str
) -> None:
    path = make_file(b"foo", name=name)

    assert b"".join(reader.bytes(path)) == b"foo"


def test_delimiter_accepts_a_control_character(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo\x7fbar")

    assert list(reader.delimiter(path, delimiter="\x7f")) == [b"foo", b"bar"]


@pytest.mark.parametrize("delimiter", ["AB", ""], ids=["two_characters", "empty"])
def test_delimiter_raises_when_not_a_single_character(
    reader: PReader, tmp_file: Path, delimiter: str
) -> None:
    with pytest.raises(ValueError, match="length 1"):
        reader.delimiter(tmp_file, delimiter=delimiter)


def test_delimiter_raises_when_missing(reader: PReader, tmp_file: Path) -> None:
    with pytest.raises(TypeError, match="delimiter"):
        reader.delimiter(tmp_file)  # type: ignore[call-arg]


def test_bytes_reads_a_long_path(
    reader: PReader, make_file: Callable[..., Path]
) -> None:
    path = make_file(b"foo", name="y" * 180 + ".bin")

    assert len(reader.bytes(path).state.name) == 64


def test_preader_states_share_the_configured_state_dir(
    config: Config, reader: PReader
) -> None:
    assert reader.states.path(TEST_STATE_NAME).parent == config.state_dir


def test_bytes_accepts_the_maximum_start_and_end(
    reader: PReader, tmp_file: Path
) -> None:
    options = IteratorOptions(start=2**64 - 1, end=2**64 - 1)

    assert list(reader.bytes(tmp_file, options=options)) == []


def test_bytes_raises_when_the_maximum_start_exceeds_the_end(
    reader: PReader, tmp_file: Path
) -> None:
    options = IteratorOptions(start=2**64 - 1, end=0)

    with pytest.raises(ValueError, match="must be <="):
        reader.bytes(tmp_file, options=options)


def test_bytes_resolves_a_relative_path_to_the_same_name(
    reader: PReader, make_file: Callable[..., Path], monkeypatch: pytest.MonkeyPatch
) -> None:
    path = make_file(b"foo")
    monkeypatch.chdir(path.parent)

    assert reader.bytes(path.name).state.name == reader.bytes(path).state.name
