"""Splits a file on a delimiter, without empty segments and with the delimiter kept."""

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
        file.write_bytes(b"seg-0,,seg-2,")

        reader = PReader()

        segments = reader.segments(file, delimiter=b",")
        filled = reader.segments(file, delimiter=b",", skip_empty=True)
        kept = reader.segments(file, delimiter=b",", keep_delimiter=True)

        logger.info("segments: %s", list(segments))
        logger.info("without empty segments: %s", list(filled))
        logger.info("with the delimiter: %s", list(kept))


if __name__ == "__main__":
    main()
