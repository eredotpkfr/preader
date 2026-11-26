"""Saves progress automatically and resumes the auto-named state in the next run."""

import logging
import tempfile

from itertools import islice
from pathlib import Path

from preader import Config, PReader

LOG_FORMAT = "%(asctime)s %(levelname)s %(message)s"

logger = logging.getLogger(__name__)


def main() -> None:
    logging.basicConfig(format=LOG_FORMAT, level=logging.INFO)

    with tempfile.TemporaryDirectory() as tmp:
        state_dir = Path(tmp, "states")
        file = Path(tmp, "data.bin")

        content = b"".join(b"line-%d\n" % index for index in range(1000))

        file.write_bytes(content)

        config = Config(
            state_dir=state_dir,
            auto_save_state=True,
            auto_save_state_bytes=1024,
            auto_load_state=True,
        )
        reader = PReader(config=config)

        read = sum(1 for _ in islice(reader.lines(file), 300))

        logger.info("first run read %d lines", read)

        resumed = reader.lines(file)

        logger.info("second run resumes at %.1f%%", resumed.percent())
        logger.info("second run starts with %s", next(resumed))

        rest = list(resumed)

        logger.info("second run ends with %s", rest[-1])
        logger.info("second run finished at %.1f%%", resumed.percent())


if __name__ == "__main__":
    main()
