"""Saves checkpoints while reading and resumes from the last one in a later run.

The line being read when a checkpoint is saved is not finished yet, so it is read again.
"""

import logging
import tempfile

from pathlib import Path

from preader import Config, PReader

LOG_FORMAT = "%(asctime)s %(levelname)s %(message)s"
CHECKPOINT_EVERY = 2
STOP_AT = 5

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

        first = reader.lines(file, state="job-1")

        for count, line in enumerate(first, start=1):
            logger.info("first run read %s", line)

            if count % CHECKPOINT_EVERY == 0:
                state = first.save()

                logger.info("checkpoint at byte %d", state.position)

            if count == STOP_AT:
                logger.warning("first run stops without a checkpoint")
                break

        for line in reader.lines(file, state="job-1"):
            logger.info("second run read %s", line)


if __name__ == "__main__":
    main()
