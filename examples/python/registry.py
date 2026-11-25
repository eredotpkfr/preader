"""Lists, searches, loads and deletes saved states, and resumes a loaded state."""

import logging
import tempfile

from pathlib import Path

from preader import Config, IteratorOptions, PReader

LOG_FORMAT = "%(asctime)s %(levelname)s %(message)s"

logger = logging.getLogger(__name__)


def save_states(reader: PReader, file: Path) -> None:
    for name in ["job-1", "sub-1/job-1", "sub-1/job-2"]:
        reader.lines(file, state=name, options=IteratorOptions(start=7)).save()

    logger.info("%d states saved", len(reader.states))


def search_states(reader: PReader) -> None:
    logger.info("under sub-1/: %s", sorted(reader.states.search("^sub-1/")))


def load_state(reader: PReader, file: Path) -> None:
    state = reader.states["job-1"]

    logger.info("job-1 resumes at byte %d", state.position)

    lines = reader.lines(file, state=state)

    logger.info("loaded job-1 resumes with %s", next(lines))


def delete_state(reader: PReader) -> None:
    del reader.states["sub-1/job-2"]

    logger.info("sub-1/job-2 is still saved: %s", "sub-1/job-2" in reader.states)


def clear_states(reader: PReader) -> None:
    reader.states.clear()

    logger.info("%d states left", len(reader.states))


def main() -> None:
    logging.basicConfig(format=LOG_FORMAT, level=logging.INFO)

    with tempfile.TemporaryDirectory() as tmp:
        state_dir = Path(tmp, "states")
        file = Path(tmp, "data.bin")

        content = b"".join(b"line-%d\n" % index for index in range(6))

        file.write_bytes(content)

        config = Config(state_dir=state_dir)
        reader = PReader(config=config)

        save_states(reader, file)
        search_states(reader)
        load_state(reader, file)
        delete_state(reader)
        clear_states(reader)


if __name__ == "__main__":
    main()
