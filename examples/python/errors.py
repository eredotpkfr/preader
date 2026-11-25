"""Handles an invalid line, an unsafe state name and a replaced file."""

import logging
import tempfile

from pathlib import Path

from preader import Config, IteratorOptions, PReader, StateMismatchError

LOG_FORMAT = "%(asctime)s %(levelname)s %(message)s"

logger = logging.getLogger(__name__)


def skip_invalid_lines(reader: PReader, file: Path) -> None:
    file.write_bytes(b"foo\n\xff\nbar\n")

    lines = reader.lines(file)

    while True:
        try:
            logger.info("read %s", next(lines))
        except ValueError as error:
            logger.warning("skipped a line: %s", error)
        except StopIteration:
            break


def reject_unsafe_name(reader: PReader, file: Path) -> None:
    try:
        reader.lines(file, state="../job-1")
    except ValueError as error:
        logger.warning("rejected the name: %s", error)


def start_over_on_a_replaced_file(reader: PReader, file: Path) -> None:
    reader.lines(file, state="job-1", options=IteratorOptions(start=4)).save()
    file.write_bytes(b"replaced\n")

    try:
        lines = reader.lines(file, state="job-1")
    except StateMismatchError as error:
        logger.warning("%s, starting over", error)

        del reader.states[error.name]
        lines = reader.lines(file, state="job-1")

    logger.info("read %s", list(lines))


def main() -> None:
    logging.basicConfig(format=LOG_FORMAT, level=logging.INFO)

    with tempfile.TemporaryDirectory() as tmp:
        state_dir = Path(tmp, "states")
        file = Path(tmp, "data.bin")

        config = Config(state_dir=state_dir)
        reader = PReader(config=config)

        skip_invalid_lines(reader, file)
        reject_unsafe_name(reader, file)
        start_over_on_a_replaced_file(reader, file)


if __name__ == "__main__":
    main()
