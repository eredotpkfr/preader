import itertools

from typing import NamedTuple

from preader import IteratorOptions

_LINE_BOUNDARY = ord("\n")


class Shape(NamedTuple):
    keep: bool
    skip_empty: bool
    align: bool


def bytes_(content: bytes, options: IteratorOptions) -> list[bytes]:
    start, end = _window(content, options)
    window = content[start:end][options.skip :][: options.limit]

    return [bytes([byte]) for byte in window]


def chunks(
    content: bytes, size: int, options: IteratorOptions, *, drop_partial: bool
) -> list[bytes]:
    start, end = _window(content, options)
    window = content[start:end]
    pieces = (
        [window[at : at + size] for at in range(0, len(window), size)] if size else []
    )
    kept = [piece for piece in pieces if not drop_partial or len(piece) == size]

    return kept[options.skip :][: options.limit]


def lines(content: bytes, shape: Shape, options: IteratorOptions) -> list[str] | None:
    try:
        return [
            line.decode()
            for line in _split(content, _LINE_BOUNDARY, shape, options, carriage=True)
        ]
    except UnicodeDecodeError:
        return None


def segments(
    content: bytes, shape: Shape, options: IteratorOptions, boundary: int
) -> list[bytes]:
    return _split(content, boundary, shape, options)


def _split(
    content: bytes,
    boundary: int,
    shape: Shape,
    options: IteratorOptions,
    *,
    carriage: bool = False,
) -> list[bytes]:
    start, end = _window(content, options)
    delimiter = bytes([boundary])
    misaligned = shape.align and start > 0 and content[start - 1] != boundary
    *whole, last = content[start:].split(delimiter)
    raws = [piece + delimiter for piece in whole] + [last] * bool(last)
    offsets = itertools.accumulate(map(len, raws), initial=start)
    raws = [raw for raw, offset in zip(raws, offsets, strict=False) if offset < end]
    items = []

    for raw in raws[options.skip + misaligned :]:
        trimmed = raw.removesuffix(delimiter)
        trimmed = trimmed.rstrip(b"\r") if carriage else trimmed

        if not (shape.skip_empty and not trimmed):
            items.append(raw if shape.keep else trimmed)

    return items[: options.limit]


def _window(content: bytes, options: IteratorOptions) -> tuple[int, int]:
    end = min(options.end, len(content))

    return min(options.start, end), end
