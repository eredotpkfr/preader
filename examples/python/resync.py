"""Resyncs a saved state after its file moved or grew, so the read goes on."""

import logging
import tempfile

from pathlib import Path

from preader import Config, LineIterator, PReader, StateOutdatedError

LOG_FORMAT = "%(asctime)s %(levelname)s %(message)s"

logger = logging.getLogger(__name__)


def resume(reader: PReader, file: Path) -> LineIterator:
    try:
        lines = reader.lines(file, state="job-1")
    except StateOutdatedError as error:
        logger.warning("%s, resyncing", error)

        reader.states.resync(error.name, file)
        lines = reader.lines(file, state="job-1")

    return lines


def main() -> None:
    logging.basicConfig(format=LOG_FORMAT, level=logging.INFO)

    with tempfile.TemporaryDirectory() as tmp:
        state_dir = Path(tmp, "states")
        file = Path(tmp, "data.bin")
        moved = Path(tmp, "moved.bin")

        file.write_bytes(b"line-0\nline-1\n")

        config = Config(state_dir=state_dir)
        reader = PReader(config=config)

        lines = reader.lines(file, state="job-1")

        for line in lines:
            logger.info("read %s", line)

        lines.save()
        file.rename(moved)

        lines = resume(reader, moved)

        logger.info("resumes at %.1f%% after the file moved", lines.percent())

        with moved.open("ab") as data:
            data.write(b"line-2\nline-3\n")

        for line in resume(reader, moved):
            logger.info("read %s after the file grew", line)


if __name__ == "__main__":
    main()
