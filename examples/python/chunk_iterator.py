"""Reads a file in fixed-size chunks, with and without the last partial chunk."""

import logging
import tempfile

from pathlib import Path

from preader import PReader

LOG_FORMAT = "%(asctime)s %(levelname)s %(message)s"

logger = logging.getLogger(__name__)


def main() -> None:
    logging.basicConfig(format=LOG_FORMAT, level=logging.INFO)

    with tempfile.TemporaryDirectory() as tmp:
        file = Path(tmp, "data.bin")
        file.write_bytes(b"abcdefghijklmnopqrstuvwxyz")

        reader = PReader()

        chunks = reader.chunks(file, chunk_size=7)
        full = reader.chunks(file, chunk_size=7, drop_partial=True)

        logger.info("chunks of 7 bytes: %s", list(chunks))
        logger.info("without the partial chunk: %s", list(full))


if __name__ == "__main__":
    main()
