import hashlib
import io
import zipfile

from pypdf import PdfWriter

from common.constants import TEST_UNICODE_TEXT

_MAX_WORDS_PER_LINE = 12
_LINE_LENGTH = 40
_MEMBER_COUNT = 3
_ATTACHMENT_NAME = "seeded.bin"
_ZIP_EPOCH = (1980, 1, 1, 0, 0, 0)
_WORDS = (
    "preader",
    "iterator",
    "state",
    "foo",
    "bar",
    "baz",
    *TEST_UNICODE_TEXT.split(),
    "$qux",
    "`quux`",
    "a;b",
    ",",
)


def seeded_content(kind: str, size: int, seed: int) -> bytes:
    match kind:
        case "random":
            return _seeded_bytes(seed, size)
        case "text":
            return _seeded_text(seed, size).encode()
        case "zip":
            return _seeded_zip(seed, size)
        case "pdf":
            return _seeded_pdf(seed, size)
        case _:
            raise ValueError(kind)


def _seeded_bytes(seed: int, size: int) -> bytes:
    return hashlib.shake_256(str(seed).encode()).digest(size)


def _seeded_text(seed: int, size: int) -> str:
    line_count = size // _LINE_LENGTH
    stream = iter(_seeded_bytes(seed, line_count * (_MAX_WORDS_PER_LINE + 1)))

    def line() -> str:
        words = next(stream) % (_MAX_WORDS_PER_LINE + 1)

        return " ".join(_WORDS[next(stream) % len(_WORDS)] for _ in range(words))

    return "".join(f"{line()}\n" for _ in range(line_count))


def _seeded_zip(seed: int, size: int) -> bytes:
    archive = io.BytesIO()

    with zipfile.ZipFile(archive, "w") as members:
        for offset in range(_MEMBER_COUNT):
            member = zipfile.ZipInfo(f"member-{offset}.bin", _ZIP_EPOCH)
            content = _seeded_bytes(seed + offset, size // _MEMBER_COUNT)
            members.writestr(member, content, zipfile.ZIP_DEFLATED)

    return archive.getvalue()


def _seeded_pdf(seed: int, size: int) -> bytes:
    document = io.BytesIO()

    with PdfWriter(document) as writer:
        writer.add_blank_page(width=612, height=792)
        writer.add_attachment(_ATTACHMENT_NAME, _seeded_bytes(seed, size))

    return document.getvalue()
