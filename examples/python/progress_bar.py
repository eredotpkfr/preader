"""Shows a tqdm progress bar that follows the read percentage and resumes with it."""

import tempfile

from pathlib import Path

from tqdm import tqdm

from preader import Config, LineIterator, PReader

BAR = "{l_bar}{bar}| {n:.1f}%"


def read_with_progress_bar(lines: LineIterator, stop: int | None = None) -> None:
    with tqdm(total=100.0, initial=lines.percent(), bar_format=BAR) as bar:
        for count, _ in enumerate(lines, start=1):
            bar.update(lines.percent() - bar.n)

            if count == stop:
                break

        bar.update(lines.percent() - bar.n)


def main() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        state_dir = Path(tmp, "states")
        file = Path(tmp, "data.bin")

        content = b"".join(b"line-%d\n" % index for index in range(200_000))

        file.write_bytes(content)

        config = Config(state_dir=state_dir)
        reader = PReader(config=config)

        first = reader.lines(file, state="job-1")

        read_with_progress_bar(first, stop=80_000)

        first.save()

        read_with_progress_bar(reader.lines(file, state="job-1"))


if __name__ == "__main__":
    main()
