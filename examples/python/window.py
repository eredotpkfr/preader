"""Reads only part of a file with start, end, skip, limit and align_start."""

import logging
import tempfile

from pathlib import Path

from preader import IteratorOptions, PReader

LOG_FORMAT = "%(asctime)s %(levelname)s %(message)s"

logger = logging.getLogger(__name__)


def main() -> None:
    logging.basicConfig(format=LOG_FORMAT, level=logging.INFO)

    with tempfile.TemporaryDirectory() as tmp:
        content = b"".join(b"line-%d\n" % index for index in range(10))

        file = Path(tmp, "data.bin")
        file.write_bytes(content)

        reader = PReader()

        middle = reader.lines(file, options=IteratorOptions(skip=1, limit=3))
        head = reader.lines(file, options=IteratorOptions(end=21))
        tail = reader.lines(file, options=IteratorOptions(start=52), align_start=True)

        logger.info("skip 1, take 3: %s", list(middle))
        logger.info("up to byte 21: %s", list(head))
        logger.info("from the first line after byte 52: %s", list(tail))


if __name__ == "__main__":
    main()
