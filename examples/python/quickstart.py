"""Reads a file line by line, follows the read percentage and saves the position."""

import logging
import tempfile

from pathlib import Path

from preader import Config, PReader

LOG_FORMAT = "%(asctime)s %(levelname)s %(message)s"

logger = logging.getLogger(__name__)


def main() -> None:
    logging.basicConfig(format=LOG_FORMAT, level=logging.INFO)

    with tempfile.TemporaryDirectory() as tmp:
        state_dir = Path(tmp, "states")
        file = Path(tmp, "data.bin")

        content = b"".join(b"line-%d\n" % index for index in range(6))

        file.write_bytes(content)

        config = Config(state_dir=state_dir)
        reader = PReader(config=config)

        for line in (lines := reader.lines(file, state="job-1")):
            logger.info("read %s, %.1f%% done", line, lines.percent())

            if line == "line-2":
                break

        state = lines.save()

        logger.info("saved %s at byte %d", state.name, state.position)


if __name__ == "__main__":
    main()
