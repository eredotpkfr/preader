"""Reads lines as they are, without empty lines and with their line endings."""

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
        file.write_bytes(b"line-0\n\nline-2\n")

        reader = PReader()

        lines = reader.lines(file)
        filled = reader.lines(file, skip_empty=True)
        ended = reader.lines(file, keepends=True)

        logger.info("lines: %s", list(lines))
        logger.info("without empty lines: %s", list(filled))
        logger.info("with line endings: %s", list(ended))


if __name__ == "__main__":
    main()
