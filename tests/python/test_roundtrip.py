import io
import itertools
import math
import random
import zipfile

import pytest

from preader import IteratorOptions

from constants import TEST_DEFAULT_DELIMITER, TEST_STATE_NAME

SEEDS = (1, 2, 3)
SIZES = (512, 4096, 51_200)
KINDS = ("random", "text", "zip")
CHUNK_SIZES = (1, 4096)
BUFFER_CAPACITIES = (1024, 65_536)
CYCLES = 5
WORDS = (
    "preader", "iterator", "state", "foo", "bar", "baz",
    "café", "Ünicode", "日本語", "🦀",
    "$qux", "`quux`", "a;b", ",",
)


def seeded_text(seed, size, line_length=40):
    rng = random.Random(seed)

    def line():
        return " ".join(rng.choice(WORDS) for _ in range(rng.randint(0, 12)))

    return "".join(f"{line()}\n" for _ in range(size // line_length))


def seeded_zip(seed, size, member_count=3, timestamp=(1980, 1, 1, 0, 0, 0)):
    archive = io.BytesIO()

    with zipfile.ZipFile(archive, "w") as members:
        for offset in range(member_count):
            member = zipfile.ZipInfo(f"member-{offset}.bin", timestamp)
            content = random.Random(seed + offset).randbytes(size // member_count)
            members.writestr(member, content, zipfile.ZIP_DEFLATED)

    return archive.getvalue()


def sized_content(kind, seed, size):
    match kind:
        case "random":
            return random.Random(seed).randbytes(size)
        case "text":
            return seeded_text(seed, size).encode()
        case _:
            return seeded_zip(seed, size)


CONTENTS = {
    f"{kind}-{size}b-seed-{seed}": sized_content(kind, seed, size)
    for kind in KINDS
    for size in SIZES
    for seed in SEEDS
}
TEXTS = {name: content.decode() for name, content in CONTENTS.items() if name.startswith("text")}
ARCHIVES = {
    name: content
    for name, content in CONTENTS.items()
    if name.startswith("zip") and name.endswith("seed-1")
}
ITERATORS = {
    "bytes": lambda reader, path, **options: reader.bytes(path, **options),
    "chunks": lambda reader, path, **options: reader.chunks(path, chunk_size=7, **options),
    "delimiter": lambda reader, path, **options: reader.delimiter(
        path, delimiter=TEST_DEFAULT_DELIMITER, keep_delimiter=True, **options
    ),
    "lines": lambda reader, path, **options: reader.lines(path, keepends=True, **options),
}


def flows(*names):
    return {
        f"{name}-{source}": (ITERATORS[name], content)
        for name in names
        for source, content in CONTENTS.items()
        if name != "lines" or source.startswith("text")
    }


FLOWS = flows(*ITERATORS)
BINARY = flows("bytes", "chunks", "delimiter")
WINDOWS = flows("bytes", "chunks")


@pytest.mark.parametrize(("make_iterator", "content"), BINARY.values(), ids=BINARY)
def test_iterators_reproduce_the_content(reader, make_file, make_iterator, content):
    assert b"".join(make_iterator(reader, make_file(content))) == content


@pytest.mark.parametrize("chunk_size", CHUNK_SIZES)
@pytest.mark.parametrize("content", CONTENTS.values(), ids=CONTENTS)
def test_chunks_reproduce_the_content(reader, make_file, content, chunk_size):
    assert b"".join(reader.chunks(make_file(content), chunk_size=chunk_size)) == content


@pytest.mark.parametrize("buffer_capacity", BUFFER_CAPACITIES)
@pytest.mark.parametrize("content", CONTENTS.values(), ids=CONTENTS)
def test_drop_partial_stops_at_the_last_whole_chunk(
    make_reader, make_file, content, buffer_capacity
):
    reader = make_reader(buffer_capacity=buffer_capacity)
    chunks = reader.chunks(make_file(content), chunk_size=7, drop_partial=True)

    assert b"".join(chunks) == content[: len(content) - len(content) % 7]


@pytest.mark.parametrize("buffer_capacity", BUFFER_CAPACITIES)
@pytest.mark.parametrize("text", TEXTS.values(), ids=TEXTS)
def test_lines_reproduce_the_text(make_reader, make_file, text, buffer_capacity):
    reader = make_reader(buffer_capacity=buffer_capacity)
    path = make_file(text.encode())

    assert "".join(reader.lines(path, keepends=True)) == text
    assert "".join(reader.lines(path)) == text.replace("\n", "")
    assert len(list(reader.lines(path))) == text.count("\n")


@pytest.mark.parametrize("content", ARCHIVES.values(), ids=ARCHIVES)
def test_lines_reject_an_archive(reader, make_file, content):
    with pytest.raises(OSError, match="valid UTF-8"):
        list(reader.lines(make_file(content)))


@pytest.mark.parametrize("text", TEXTS.values(), ids=TEXTS)
def test_skip_empty_drops_the_blank_lines(reader, make_file, text):
    kept = list(reader.lines(make_file(text.encode()), skip_empty=True))

    assert kept == [line for line in text.splitlines() if line]


@pytest.mark.parametrize(("make_iterator", "content"), FLOWS.values(), ids=FLOWS)
def test_percent_tracks_the_position(reader, make_file, make_iterator, content):
    iterator = make_iterator(reader, make_file(content))

    for _ in iterator:
        assert iterator.percent() == pytest.approx(iterator.state.position / len(content) * 100)

    assert iterator.state.position == len(content)
    assert iterator.percent() == 100.0


@pytest.mark.parametrize(("make_iterator", "content"), FLOWS.values(), ids=FLOWS)
def test_resume_rebuilds_the_file_in_cycles(
    reader, make_reader, make_file, make_iterator, content
):
    resuming = make_reader(auto_load_state=True, auto_save_state=True)
    path = make_file(content)
    expected = list(make_iterator(reader, path))
    cycle = math.ceil(len(expected) / (CYCLES - 1))

    rebuilt = [
        item
        for _ in range(CYCLES)
        for item in itertools.islice(make_iterator(resuming, path, state=TEST_STATE_NAME), cycle)
    ]

    assert rebuilt == expected


@pytest.mark.parametrize(("make_iterator", "content"), WINDOWS.values(), ids=WINDOWS)
def test_options_window_the_byte_range(reader, make_file, make_iterator, content):
    options = IteratorOptions(start=100, end=500)
    windowed = make_iterator(reader, make_file(content), options=options)

    assert b"".join(windowed) == content[100:500]
