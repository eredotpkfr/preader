from hypothesis import strategies as st

from common.matrix import contents
from preader import Config, IteratorOptions

_UNBOUNDED = IteratorOptions().end
_HANDWRITTEN = [
    contents.EMPTY,
    contents.LINES,
    contents.UNTERMINATED_LINES,
    contents.CRLF_LINES,
    contents.BLANK_LINES,
    contents.SEGMENTS,
    contents.UNTERMINATED_SEGMENTS,
    contents.BLANK_SEGMENTS,
    contents.WHOLE_SEGMENT,
    contents.MISALIGNED_SEGMENTS,
    contents.UTF8_SEGMENTS,
    contents.ALL_BYTES,
]
_DENSE = b",\n\r\x00x\x7f\x80\xe9\xff"
_LETTERS = ["a", ",", "\n", "\r", "é", "🦀"]
_MAX_BYTES = 64
_MAX_LETTERS = 16
_MAX_CAPACITY = 16
_BINARY = st.lists(
    st.sampled_from(_DENSE) | st.integers(0, 255), max_size=_MAX_BYTES
).map(bytes)
_TEXT = st.lists(st.sampled_from(_LETTERS), max_size=_MAX_LETTERS).map(
    lambda letters: "".join(letters).encode()
)
_CONTENTS = st.sampled_from(_HANDWRITTEN) | _BINARY | _TEXT
_TEXTS = st.sampled_from(_HANDWRITTEN) | _TEXT
_CAPACITIES = st.integers(1, _MAX_CAPACITY) | st.just(Config().buffer_capacity)


@st.composite
def bytes_(draw: st.DrawFn) -> tuple[bytes, IteratorOptions, int]:
    content = draw(_CONTENTS)

    return content, draw(_options(len(content), len(content))), draw(_CAPACITIES)


@st.composite
def chunks(draw: st.DrawFn) -> tuple[bytes, IteratorOptions, int, int]:
    content = draw(_CONTENTS)
    size = draw(st.integers(0, len(content) + 2))
    items = len(content) // size + 1 if size else 0

    return content, draw(_options(len(content), items)), size, draw(_CAPACITIES)


def lines() -> st.SearchStrategy[tuple[bytes, IteratorOptions, int]]:
    return _delimited(_TEXTS, ord("\n"))


def segments(delimiter: int) -> st.SearchStrategy[tuple[bytes, IteratorOptions, int]]:
    return _delimited(_CONTENTS, delimiter)


@st.composite
def _delimited(
    draw: st.DrawFn, contents: st.SearchStrategy[bytes], delimiter: int
) -> tuple[bytes, IteratorOptions, int]:
    content = draw(contents)
    items = content.count(delimiter) + 1

    return content, draw(_options(len(content), items)), draw(_CAPACITIES)


@st.composite
def _options(draw: st.DrawFn, size: int, items: int) -> IteratorOptions:
    start, end = draw(_bounds(size))

    return IteratorOptions(
        start=start,
        end=end,
        skip=draw(st.integers(0, items + 1)),
        limit=draw(_position(items)),
    )


def _bounds(size: int) -> st.SearchStrategy[tuple[int, int]]:
    ordered = st.tuples(_position(size), _position(size)).map(
        lambda pair: (min(pair), max(pair))
    )
    inverted = st.tuples(st.integers(0, size), st.integers(0, size)).map(
        lambda pair: (max(pair) + 1, min(pair))
    )

    return _weighted(ordered, inverted)


def _position(size: int) -> st.SearchStrategy[int]:
    return _weighted(st.integers(0, size + 1), st.just(_UNBOUNDED))


@st.composite
def _weighted[T](
    draw: st.DrawFn, common: st.SearchStrategy[T], rare: st.SearchStrategy[T]
) -> T:
    return draw(rare if draw(st.integers(0, 9)) == 9 else common)
