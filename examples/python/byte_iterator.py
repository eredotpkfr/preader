"""Reads a file byte by byte, then only a window of it."""

import logging
import tempfile

from pathlib import Path

from preader import IteratorOptions, PReader

LOG_FORMAT = "%(asctime)s %(levelname)s %(message)s"

logger = logging.getLogger(__name__)


def main() -> None:
    logging.basicConfig(format=LOG_FORMAT, level=logging.INFO)

    with tempfile.TemporaryDirectory() as tmp:
        file = Path(tmp, "data.bin")
        file.write_bytes(b"abcdefghijklmnopqrstuvwxyz")

        reader = PReader()

        every = b"".join(reader.bytes(file))
        window = b"".join(reader.bytes(file, options=IteratorOptions(skip=3, limit=4)))

        logger.info("every byte: %s", every)
        logger.info("4 bytes after the first 3: %s", window)


if __name__ == "__main__":
    main()
