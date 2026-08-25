import pytest

from preader import Config, IteratorOptions, PReader, StateError

from constants import TEST_ALPHABET, TEST_STATE_NAME


def test_preader_defaults():
    config = PReader().config
    default = Config()

    assert config.buffer_capacity == default.buffer_capacity
    assert config.state_dir == default.state_dir
    assert config.auto_save_state == default.auto_save_state
    assert config.auto_save_state_bytes == default.auto_save_state_bytes
    assert config.auto_load_state == default.auto_load_state
    assert config.verify_state == default.verify_state


def test_raises_when_config_is_none():
    with pytest.raises(TypeError):
        PReader(config=None)


def test_buffer_capacity_zero(make_reader, tmp_file):
    reader = make_reader(buffer_capacity=0)

    assert b"".join(reader.bytes(tmp_file)) == tmp_file.read_bytes()


def test_bytes_raises_when_file_missing(reader, tmp_path):
    with pytest.raises(FileNotFoundError):
        reader.bytes(tmp_path / "does-not-exist.bin")


def test_preader_repr(config, reader, reindent, expected_repr):
    assert repr(reader) == expected_repr("PReader", config=reindent(repr(config), 2))


@pytest.mark.usefixtures("requires_symlinks")
def test_bytes_follows_symlink(reader, tmp_file):
    symlink = tmp_file.parent / "link.bin"
    symlink.symlink_to(tmp_file)

    state = reader.bytes(symlink).state

    assert state.file.path == tmp_file


def test_bytes_raises_when_path_is_a_directory(reader, tmp_path):
    directory = tmp_path / "a-directory"
    directory.mkdir()

    with pytest.raises(StateError, match="not a file"):
        reader.bytes(directory)


def test_bytes_yields_nothing_for_empty_file(reader, empty_file):
    assert list(reader.bytes(empty_file)) == []


def test_bytes_yields_one_item_for_a_single_byte_file(reader, make_file):
    assert list(reader.bytes(make_file(b"a"))) == [b"a"]


def test_raises_when_start_exceeds_end(reader, tmp_file):
    with pytest.raises(ValueError, match="start .* must be <= end"):
        options = IteratorOptions(start=10, end=5)

        reader.bytes(tmp_file, options=options)


def test_chunks_splits_into_fixed_size_pieces(reader, make_file):
    path = make_file(TEST_ALPHABET)
    chunk_size = 3

    chunks = list(reader.chunks(path, chunk_size=chunk_size))
    expected = [TEST_ALPHABET[i : i + chunk_size] for i in range(0, len(TEST_ALPHABET), chunk_size)]

    assert chunks == expected


def test_chunks_drop_partial_discards_partial_chunk(reader, make_file):
    path = make_file(TEST_ALPHABET)
    chunk_size = 3

    chunks = list(reader.chunks(path, chunk_size=chunk_size, drop_partial=True))

    full_length = (len(TEST_ALPHABET) // chunk_size) * chunk_size
    expected = [TEST_ALPHABET[i : i + chunk_size] for i in range(0, full_length, chunk_size)]

    assert chunks == expected


def test_chunks_drop_partial_keeps_exact_chunk(reader, make_file):
    path = make_file(b"abcdef")

    assert list(reader.chunks(path, chunk_size=3, drop_partial=True)) == [b"abc", b"def"]


def test_chunks_single_chunk_for_large_chunk_size(reader, tmp_file):
    assert list(reader.chunks(tmp_file, chunk_size=1024)) == [tmp_file.read_bytes()]


def test_chunks_drop_partial_empty_for_large_chunk_size(reader, tmp_file):
    assert list(reader.chunks(tmp_file, chunk_size=1024, drop_partial=True)) == []


@pytest.mark.parametrize("drop_partial", [True, False], ids=["drop_partial", "defaults"])
def test_chunks_yields_nothing_for_empty_file(reader, empty_file, drop_partial):
    assert list(reader.chunks(empty_file, drop_partial=drop_partial)) == []


@pytest.mark.parametrize(
    ("drop_partial", "expected"), [(True, []), (False, [b"a"])], ids=["drop_partial", "defaults"]
)
def test_chunks_yields_one_item_for_a_single_byte_file(reader, make_file, drop_partial, expected):
    assert list(reader.chunks(make_file(b"a"), drop_partial=drop_partial)) == expected


@pytest.mark.parametrize("drop_partial", [True, False], ids=["drop_partial", "defaults"])
def test_chunks_attributes(reader, tmp_file, drop_partial):
    iterator = reader.chunks(tmp_file, chunk_size=1024, drop_partial=drop_partial)

    assert iterator.chunk_size == 1024
    assert iterator.drop_partial is drop_partial


def test_lines_splits_on_newline(reader, make_file):
    path = make_file(b"foo\nbar\nbaz\n")

    assert list(reader.lines(path)) == ["foo", "bar", "baz"]


def test_lines_keepends_preserves_newline(reader, make_file):
    path = make_file(b"foo\nbar\n")

    assert list(reader.lines(path, keepends=True)) == ["foo\n", "bar\n"]


def test_lines_keepends_keeps_blank_lines(reader, make_file):
    path = make_file(b"foo\n\nbar\n")

    assert list(reader.lines(path, keepends=True)) == ["foo\n", "\n", "bar\n"]


def test_lines_yields_nothing_for_empty_file(reader, empty_file):
    assert list(reader.lines(empty_file)) == []


def test_lines_yields_one_item_for_a_single_byte_file(reader, make_file):
    assert list(reader.lines(make_file(b"a"))) == ["a"]


def test_lines_keeps_blank_lines_by_default(reader, make_file):
    path = make_file(b"foo\n\nbar\n")

    assert list(reader.lines(path)) == ["foo", "", "bar"]


def test_lines_skip_empty_skips_blank_lines(reader, make_file):
    path = make_file(b"foo\n\nbar\n")

    assert list(reader.lines(path, skip_empty=True)) == ["foo", "bar"]


def test_lines_skip_empty_with_keepends(reader, make_file):
    path = make_file(b"foo\n\nbar\n")

    assert list(reader.lines(path, keepends=True, skip_empty=True)) == ["foo\n", "bar\n"]


@pytest.mark.parametrize(
    ("keepends", "skip_empty"),
    [(True, True), (True, False), (False, True), (False, False)],
    ids=["both", "keepends", "skip_empty", "defaults"],
)
def test_lines_attributes(reader, tmp_file, keepends, skip_empty):
    iterator = reader.lines(tmp_file, keepends=keepends, skip_empty=skip_empty)

    assert iterator.keepends is keepends
    assert iterator.skip_empty is skip_empty


def test_delimiter_splits_on_byte(reader, make_file):
    path = make_file(b"foo,bar,baz")

    assert list(reader.delimiter(path, delimiter=",")) == [b"foo", b"bar", b"baz"]


def test_delimiter_keep_delimiter_preserves_byte(reader, make_file):
    path = make_file(b"foo,bar,")

    assert list(reader.delimiter(path, delimiter=",", keep_delimiter=True)) == [b"foo,", b"bar,"]


def test_delimiter_keep_delimiter_keeps_blank_segments(reader, make_file):
    path = make_file(b"foo,,bar,")

    segments = list(reader.delimiter(path, delimiter=",", keep_delimiter=True))

    assert segments == [b"foo,", b",", b"bar,"]


def test_delimiter_yields_nothing_for_empty_file(reader, empty_file):
    assert list(reader.delimiter(empty_file, delimiter=",")) == []


def test_delimiter_yields_one_item_for_a_single_byte_file(reader, make_file):
    assert list(reader.delimiter(make_file(b"a"), delimiter=",")) == [b"a"]


def test_delimiter_keeps_blank_segments_by_default(reader, make_file):
    path = make_file(b"foo,,bar,")

    assert list(reader.delimiter(path, delimiter=",")) == [b"foo", b"", b"bar"]


def test_delimiter_skip_empty_skips_blank_segments(reader, make_file):
    path = make_file(b"foo,,bar,")

    assert list(reader.delimiter(path, delimiter=",", skip_empty=True)) == [b"foo", b"bar"]


def test_delimiter_skip_empty_with_keep_delimiter(reader, make_file):
    path = make_file(b"foo,,bar,")

    segments = list(reader.delimiter(path, delimiter=",", keep_delimiter=True, skip_empty=True))

    assert segments == [b"foo,", b"bar,"]


@pytest.mark.parametrize(
    ("keep_delimiter", "skip_empty"),
    [(True, True), (True, False), (False, True), (False, False)],
    ids=["both", "keep_delimiter", "skip_empty", "defaults"],
)
def test_delimiter_attributes(reader, tmp_file, keep_delimiter, skip_empty):
    iterator = reader.delimiter(
        tmp_file, delimiter=",", keep_delimiter=keep_delimiter, skip_empty=skip_empty
    )

    assert iterator.delimiter == ord(",")
    assert iterator.keep_delimiter is keep_delimiter
    assert iterator.skip_empty is skip_empty


def test_delimiter_accepts_codepoint_up_to_255(reader, make_file):
    path = make_file(b"foo" + bytes([ord("é")]) + b"bar")

    assert list(reader.delimiter(path, delimiter="é")) == [b"foo", b"bar"]


def test_delimiter_raises_when_codepoint_above_255(reader, tmp_file):
    with pytest.raises(ValueError, match="delimiter must fit in a single byte"):
        reader.delimiter(tmp_file, delimiter="€")


def test_bytes_reads_a_unicode_path(reader, make_file):
    path = make_file(b"foo", name="café-日本語.bin")

    assert b"".join(reader.bytes(path)) == b"foo"


def test_bytes_reads_a_path_with_special_characters(reader, make_file):
    path = make_file(b"foo", name="foo bar; $baz & `qux`.bin")

    assert b"".join(reader.bytes(path)) == b"foo"


def test_delimiter_accepts_a_control_character(reader, make_file):
    path = make_file(b"foo\x7fbar")

    assert list(reader.delimiter(path, delimiter="\x7f")) == [b"foo", b"bar"]


@pytest.mark.parametrize("delimiter", ["AB", ""], ids=["two_characters", "empty"])
def test_delimiter_raises_when_not_a_single_character(reader, tmp_file, delimiter):
    with pytest.raises(ValueError, match="length 1"):
        reader.delimiter(tmp_file, delimiter=delimiter)


def test_delimiter_raises_when_missing(reader, tmp_file):
    with pytest.raises(TypeError, match="delimiter"):
        reader.delimiter(tmp_file)


def test_bytes_reads_a_long_path(reader, make_file):
    path = make_file(b"foo", name="y" * 180 + ".bin")

    assert len(reader.bytes(path).state.name) == 64


def test_preader_states_share_the_configured_state_dir(config, reader):
    assert reader.states.path(TEST_STATE_NAME).parent == config.state_dir


def test_bytes_accepts_the_maximum_start_and_end(reader, tmp_file):
    options = IteratorOptions(start=2**64 - 1, end=2**64 - 1)

    assert list(reader.bytes(tmp_file, options=options)) == []


def test_bytes_raises_when_the_maximum_start_exceeds_the_end(reader, tmp_file):
    options = IteratorOptions(start=2**64 - 1, end=0)

    with pytest.raises(ValueError, match="must be <="):
        reader.bytes(tmp_file, options=options)


def test_bytes_resolves_a_relative_path_to_the_same_name(reader, make_file, monkeypatch):
    path = make_file(b"foo")
    monkeypatch.chdir(path.parent)

    assert reader.bytes(path.name).state.name == reader.bytes(path).state.name
