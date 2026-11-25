"""
A file reader that tracks read percentage and manages read state
"""

from _typeshed import SupportsLenAndGetItem
from datetime import datetime
from os import PathLike
from pathlib import Path
from typing import SupportsIndex, final
from typing_extensions import disjoint_base

__all__ = ["ByteIterator", "ChunkIterator", "Config", "FileMetadata", "IteratorBase", "IteratorOptions", "LineIterator", "PReader", "SegmentIterator", "State", "StateConflictError", "StateCorruptError", "StateError", "StateIterator", "StateMismatchError", "StateOutdatedError", "StateRegistry", "Timestamps"]

@final
class ByteIterator(IteratorBase):
    """
    Reads a file one byte at a time, yielding each byte as a one-byte `bytes` object

    Create one with `reader.bytes(file)`
    """
    def __iter__(self, /) -> ByteIterator:
        """
        Returns the iterator itself
        """
    def __next__(self, /) -> bytes:
        """
        Returns the next item and raises `StopIteration` once the read is finished

        Raises `OSError` when reading the file fails and, for `LineIterator`, `ValueError`
        when a line is not valid UTF-8; iteration can continue after either, except after an
        `OSError` that read nothing, which ends it. Like any item, a failed one is finished
        only once the next item is requested, so a state saved right after the error resumes
        at it and raises it again. An invalid line also counts toward `options.limit`. When
        `auto_save_state` is on, a failed autosave raises the errors of `save()` and ends
        the iteration
        """
    def __repr__(self, /) -> str:
        """
        Returns the class name with the state and the iterator settings
        """
    def percent(self, /) -> float:
        """
        Returns the share of the file finished so far, from 0.0 to 100.0

        It counts finished items, so it stays below 100.0 while the last item is unfinished.
        It reaches 100.0 only when the read reaches the end of the file; `options.end`,
        `options.limit` or, for `ChunkIterator`, `drop_partial` stop it earlier
        """
    def save(self, /) -> State:
        """
        Saves the current state to the reader's state directory and returns it

        The state holds finished items only, so unless the iteration has ended the last
        yielded item is read again on resume. It works whether or not `auto_save_state` is
        on, and a failed save does not stop the iteration. Raises `StateConflictError` when
        another writer saved the state since this iterator loaded or last saved it,
        `StateError` when the state name has become an alias, and `OSError` when the state
        file cannot be written. An iterator that did not start from the saved state
        overwrites it on its first save. A saved state that was deleted, cannot be read,
        does not parse or belongs to another name is always written over. A state is meant
        to have one writer at a time, so concurrent writers of one state must be coordinated
        by the caller
        """
    @property
    def state(self, /) -> State:
        """
        Copy of the current state, positioned at the end of the last finished item

        An item counts as finished once the next one is requested, so a state saved right
        after reading an item resumes at that item. With `verify_state` on, passing a copy
        as `state` raises `StateMismatchError` when the position moved after the state was
        created, loaded or saved. Reading, `options.start` and a byte or chunk
        `options.skip` move the position, so pass the `State` that `save()` returned instead
        """

@final
class ChunkIterator(IteratorBase):
    """
    Reads a file in fixed-size chunks, yielding each chunk as `bytes`

    Create one with `reader.chunks(file, chunk_size=1024)`. The last chunk is shorter when the
    read range does not divide evenly, and `drop_partial` drops it
    """
    def __iter__(self, /) -> ChunkIterator:
        """
        Returns the iterator itself
        """
    def __next__(self, /) -> bytes:
        """
        Returns the next item and raises `StopIteration` once the read is finished

        Raises `OSError` when reading the file fails and, for `LineIterator`, `ValueError`
        when a line is not valid UTF-8; iteration can continue after either, except after an
        `OSError` that read nothing, which ends it. Like any item, a failed one is finished
        only once the next item is requested, so a state saved right after the error resumes
        at it and raises it again. An invalid line also counts toward `options.limit`. When
        `auto_save_state` is on, a failed autosave raises the errors of `save()` and ends
        the iteration
        """
    def __repr__(self, /) -> str:
        """
        Returns the class name with the state and the iterator settings
        """
    @property
    def chunk_size(self, /) -> int:
        """
        Size in bytes of every chunk but the last; 0 yields no chunks
        """
    @property
    def drop_partial(self, /) -> bool:
        """
        Whether a last chunk shorter than `chunk_size` is dropped
        """
    def percent(self, /) -> float:
        """
        Returns the share of the file finished so far, from 0.0 to 100.0

        It counts finished items, so it stays below 100.0 while the last item is unfinished.
        It reaches 100.0 only when the read reaches the end of the file; `options.end`,
        `options.limit` or, for `ChunkIterator`, `drop_partial` stop it earlier
        """
    def save(self, /) -> State:
        """
        Saves the current state to the reader's state directory and returns it

        The state holds finished items only, so unless the iteration has ended the last
        yielded item is read again on resume. It works whether or not `auto_save_state` is
        on, and a failed save does not stop the iteration. Raises `StateConflictError` when
        another writer saved the state since this iterator loaded or last saved it,
        `StateError` when the state name has become an alias, and `OSError` when the state
        file cannot be written. An iterator that did not start from the saved state
        overwrites it on its first save. A saved state that was deleted, cannot be read,
        does not parse or belongs to another name is always written over. A state is meant
        to have one writer at a time, so concurrent writers of one state must be coordinated
        by the caller
        """
    @property
    def state(self, /) -> State:
        """
        Copy of the current state, positioned at the end of the last finished item

        An item counts as finished once the next one is requested, so a state saved right
        after reading an item resumes at that item. With `verify_state` on, passing a copy
        as `state` raises `StateMismatchError` when the position moved after the state was
        created, loaded or saved. Reading, `options.start` and a byte or chunk
        `options.skip` move the position, so pass the `State` that `save()` returned instead
        """

@final
class Config:
    """
    Settings for buffering and for where and when states are saved, loaded and verified
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(cls, /, *, buffer_capacity: SupportsIndex = ..., state_dir: str | PathLike[str] = ..., auto_save_state: bool = False, auto_save_state_bytes: SupportsIndex = ..., auto_load_state: bool = False, verify_state: bool = ...) -> Config:
        """
        Creates a configuration from keyword arguments; omitted ones keep their defaults

        By default states live in a `preader` directory under the user's cache directory, the read
        buffer holds 64 KiB and `verify_state` is on. Autosave runs every 100 MiB once
        `auto_save_state` is on. Raises `ValueError` when `auto_save_state_bytes` is 0
        """
    def __repr__(self, /) -> str:
        """
        Returns the class name with every setting
        """
    @property
    def auto_load_state(self, /) -> bool:
        """
        Whether an iterator created without a state resumes its auto-named state
        """
    @property
    def auto_save_state(self, /) -> bool:
        """
        Whether iterators save every `auto_save_state_bytes`, at the end and when discarded
        """
    @property
    def auto_save_state_bytes(self, /) -> int:
        """
        Bytes of progress between two autosaves while `auto_save_state` is on
        """
    @property
    def buffer_capacity(self, /) -> int:
        """
        Capacity of the read buffer in bytes, where `0` is treated as `1`
        """
    @property
    def state_dir(self, /) -> Path:
        """
        Directory states are saved in, or the current directory when empty
        """
    @property
    def verify_state(self, /) -> bool:
        """
        Whether a resumed or loaded state is checked for edits and for changes to its file
        """

@final
class FileMetadata:
    """
    Path, size, modification time and fingerprint of a state's tracked file

    The metadata is recorded when the state is created or resynced and is never refreshed by a
    save. After the file grows, resuming reads only up to `size`, or fails with a size mismatch
    when `verify_state` is on, until a resync records the file again
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str:
        """
        Returns the class name with the path, size, mtime and fingerprint
        """
    @property
    def fingerprint(self, /) -> str:
        """
        SHA-256 hex digest of the first 4096 bytes of the file
        """
    @property
    def mtime(self, /) -> datetime:
        """
        Modification time of the file, at microsecond precision
        """
    @property
    def path(self, /) -> Path:
        """
        Path of the file, canonical when recorded by an iterator or a resync
        """
    @property
    def size(self, /) -> int:
        """
        Size of the file in bytes, where reads stop
        """

@disjoint_base
class IteratorBase:
    """
    Common base class of `ByteIterator`, `ChunkIterator`, `LineIterator` and `SegmentIterator`

    It has no members of its own; use it with `isinstance` to accept any preader file iterator
    """

@final
class IteratorOptions:
    """
    Byte range, skip count and item limit that select which part of a file an iterator reads
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __new__(cls, /, *, start: SupportsIndex = 0, end: SupportsIndex = ..., skip: SupportsIndex = 0, limit: SupportsIndex = ...) -> IteratorOptions:
        """
        Creates iterator options from keyword arguments; omitted ones read the whole file

        `start` and `end` are byte offsets and `limit` is the most items one iterator yields.
        `skip` moves `start` forward by that many bytes or chunks, while lines and segments are
        skipped only when a resumed state is not past `start`. `start` greater than `end` raises
        `ValueError` when the iterator is created, not here
        """
    def __repr__(self, /) -> str:
        """
        Returns the class name with every option
        """
    @property
    def end(self, /) -> int:
        """
        Byte offset at which no new item starts, clamped to the recorded file size
        """
    @property
    def limit(self, /) -> int:
        """
        Maximum number of items one iterator yields
        """
    @property
    def skip(self, /) -> int:
        """
        Number of items skipped after `start`, counted in bytes by byte iterators
        """
    @property
    def start(self, /) -> int:
        """
        Byte offset where reading begins
        """

@final
class LineIterator(IteratorBase):
    """
    Reads a UTF-8 file line by line, yielding each line as `str`

    Create one with `reader.lines(file)`. Only a newline ends a line; the whole trailing run of
    newline and carriage return characters is removed unless `keepends` is set. A last line
    without a newline is not yielded when the file has grown past its recorded size, since
    its end is not known yet
    """
    def __iter__(self, /) -> LineIterator:
        """
        Returns the iterator itself
        """
    def __next__(self, /) -> str:
        """
        Returns the next item and raises `StopIteration` once the read is finished

        Raises `OSError` when reading the file fails and, for `LineIterator`, `ValueError`
        when a line is not valid UTF-8; iteration can continue after either, except after an
        `OSError` that read nothing, which ends it. Like any item, a failed one is finished
        only once the next item is requested, so a state saved right after the error resumes
        at it and raises it again. An invalid line also counts toward `options.limit`. When
        `auto_save_state` is on, a failed autosave raises the errors of `save()` and ends
        the iteration
        """
    def __repr__(self, /) -> str:
        """
        Returns the class name with the state and the iterator settings
        """
    @property
    def keepends(self, /) -> bool:
        """
        Whether each line keeps its trailing newline and carriage return characters
        """
    def percent(self, /) -> float:
        """
        Returns the share of the file finished so far, from 0.0 to 100.0

        It counts finished items, so it stays below 100.0 while the last item is unfinished.
        It reaches 100.0 only when the read reaches the end of the file; `options.end`,
        `options.limit` or, for `ChunkIterator`, `drop_partial` stop it earlier
        """
    def save(self, /) -> State:
        """
        Saves the current state to the reader's state directory and returns it

        The state holds finished items only, so unless the iteration has ended the last
        yielded item is read again on resume. It works whether or not `auto_save_state` is
        on, and a failed save does not stop the iteration. Raises `StateConflictError` when
        another writer saved the state since this iterator loaded or last saved it,
        `StateError` when the state name has become an alias, and `OSError` when the state
        file cannot be written. An iterator that did not start from the saved state
        overwrites it on its first save. A saved state that was deleted, cannot be read,
        does not parse or belongs to another name is always written over. A state is meant
        to have one writer at a time, so concurrent writers of one state must be coordinated
        by the caller
        """
    @property
    def skip_empty(self, /) -> bool:
        """
        Whether lines that are empty once the terminator is removed are skipped
        """
    @property
    def skip_remaining(self, /) -> int:
        """
        Number of lines still to be skipped before the next one is yielded

        With `align_start`, it includes the partial line skipped at `options.start`
        """
    @property
    def state(self, /) -> State:
        """
        Copy of the current state, positioned at the end of the last finished item

        An item counts as finished once the next one is requested, so a state saved right
        after reading an item resumes at that item. With `verify_state` on, passing a copy
        as `state` raises `StateMismatchError` when the position moved after the state was
        created, loaded or saved. Reading, `options.start` and a byte or chunk
        `options.skip` move the position, so pass the `State` that `save()` returned instead
        """

@final
class PReader:
    """
    Entry point that reads files as resumable iterators

    A reader owns a [`Config`] and the [`StateRegistry`] for its state directory, and reads a
    file as bytes, chunks, lines or segments
    """
    def __new__(cls, /, *, config: Config = ...) -> PReader:
        """
        Creates a reader that uses `config`, or a default `Config()` when it is omitted
        """
    def __repr__(self, /) -> str:
        """
        Returns the class name with the configuration
        """
    def bytes(self, /, file: str | PathLike[str], *, state: State | str | None = ..., options: IteratorOptions = ...) -> ByteIterator:
        """
        Returns a `ByteIterator` that reads `file` one byte at a time

        A state name resumes the state saved under it, or starts fresh when none is saved, and a
        `State` resumes from itself. `state=None` uses the auto-named state of the file, which
        resumes only with `auto_load_state`. An iterator that does not start from the saved state
        overwrites it on its first save. Raises `OSError` when the file or the saved state cannot be
        read, `ValueError` for an invalid state name or `options.start` greater than `options.end`,
        `StateError` for an aliased state name, and one of its subclasses when the saved or passed
        state is corrupt or cannot resume on the file
        """
    def chunks(self, /, file: str | PathLike[str], *, state: State | str | None = ..., options: IteratorOptions = ..., chunk_size: SupportsIndex = ..., drop_partial: bool = False) -> ChunkIterator:
        """
        Returns a `ChunkIterator` that reads `file` in chunks of `chunk_size` bytes, 1024 by default

        A state name resumes the state saved under it, or starts fresh when none is saved, and a
        `State` resumes from itself. `state=None` uses the auto-named state of the file, which
        resumes only with `auto_load_state`. An iterator that does not start from the saved state
        overwrites it on its first save. Raises `OSError` when the file or the saved state cannot be
        read, `ValueError` for an invalid state name or `options.start` greater than `options.end`,
        `StateError` for an aliased state name, and one of its subclasses when the saved or passed
        state is corrupt or cannot resume on the file
        """
    @property
    def config(self, /) -> Config:
        """
        Copy of the configuration the reader was created with
        """
    def lines(self, /, file: str | PathLike[str], *, state: State | str | None = ..., options: IteratorOptions = ..., keepends: bool = False, align_start: bool = False, skip_empty: bool = False) -> LineIterator:
        """
        Returns a `LineIterator` that reads `file` as UTF-8 lines

        With `align_start`, a partial line at `options.start` is skipped unless a resumed state is
        already past it

        A state name resumes the state saved under it, or starts fresh when none is saved, and a
        `State` resumes from itself. `state=None` uses the auto-named state of the file, which
        resumes only with `auto_load_state`. An iterator that does not start from the saved state
        overwrites it on its first save. Raises `OSError` when the file or the saved state cannot be
        read, `ValueError` for an invalid state name or `options.start` greater than `options.end`,
        `StateError` for an aliased state name, and one of its subclasses when the saved or passed
        state is corrupt or cannot resume on the file
        """
    def segments(self, /, file: str | PathLike[str], *, state: State | str | None = ..., options: IteratorOptions = ..., delimiter: SupportsLenAndGetItem[SupportsIndex] = ..., keep_delimiter: bool = False, align_start: bool = False, skip_empty: bool = False) -> SegmentIterator:
        """
        Returns a `SegmentIterator` that reads `file` as segments ended by `delimiter`

        `delimiter` must be a one-byte `bytes` and defaults to `b","`; any other length raises
        `ValueError` and a one-character `str` raises `TypeError`. With `align_start`, a partial
        segment at `options.start` is skipped unless a resumed state is already past it

        A state name resumes the state saved under it, or starts fresh when none is saved, and a
        `State` resumes from itself. `state=None` uses the auto-named state of the file, which
        resumes only with `auto_load_state`. An iterator that does not start from the saved state
        overwrites it on its first save. Raises `OSError` when the file or the saved state cannot be
        read, `ValueError` for an invalid state name or `options.start` greater than `options.end`,
        `StateError` for an aliased state name, and one of its subclasses when the saved or passed
        state is corrupt or cannot resume on the file
        """
    @property
    def states(self, /) -> StateRegistry:
        """
        Registry of the states saved in this reader's state directory
        """

@final
class SegmentIterator(IteratorBase):
    """
    Reads a file split on a one-byte delimiter, yielding each segment as `bytes`

    Create one with `reader.segments(file, delimiter=b",")`. One trailing delimiter is removed
    from each segment unless `keep_delimiter` is set. A last segment without the delimiter is
    not yielded when the file has grown past its recorded size, since its end is not known yet
    """
    def __iter__(self, /) -> SegmentIterator:
        """
        Returns the iterator itself
        """
    def __next__(self, /) -> bytes:
        """
        Returns the next item and raises `StopIteration` once the read is finished

        Raises `OSError` when reading the file fails and, for `LineIterator`, `ValueError`
        when a line is not valid UTF-8; iteration can continue after either, except after an
        `OSError` that read nothing, which ends it. Like any item, a failed one is finished
        only once the next item is requested, so a state saved right after the error resumes
        at it and raises it again. An invalid line also counts toward `options.limit`. When
        `auto_save_state` is on, a failed autosave raises the errors of `save()` and ends
        the iteration
        """
    def __repr__(self, /) -> str:
        """
        Returns the class name with the state and the iterator settings
        """
    @property
    def delimiter(self, /) -> bytes:
        """
        One-byte delimiter that ends each segment
        """
    @property
    def keep_delimiter(self, /) -> bool:
        """
        Whether each segment keeps its trailing delimiter
        """
    def percent(self, /) -> float:
        """
        Returns the share of the file finished so far, from 0.0 to 100.0

        It counts finished items, so it stays below 100.0 while the last item is unfinished.
        It reaches 100.0 only when the read reaches the end of the file; `options.end`,
        `options.limit` or, for `ChunkIterator`, `drop_partial` stop it earlier
        """
    def save(self, /) -> State:
        """
        Saves the current state to the reader's state directory and returns it

        The state holds finished items only, so unless the iteration has ended the last
        yielded item is read again on resume. It works whether or not `auto_save_state` is
        on, and a failed save does not stop the iteration. Raises `StateConflictError` when
        another writer saved the state since this iterator loaded or last saved it,
        `StateError` when the state name has become an alias, and `OSError` when the state
        file cannot be written. An iterator that did not start from the saved state
        overwrites it on its first save. A saved state that was deleted, cannot be read,
        does not parse or belongs to another name is always written over. A state is meant
        to have one writer at a time, so concurrent writers of one state must be coordinated
        by the caller
        """
    @property
    def skip_empty(self, /) -> bool:
        """
        Whether segments that are empty once the delimiter is removed are skipped
        """
    @property
    def skip_remaining(self, /) -> int:
        """
        Number of segments still to be skipped before the next one is yielded

        With `align_start`, it includes the partial segment skipped at `options.start`
        """
    @property
    def state(self, /) -> State:
        """
        Copy of the current state, positioned at the end of the last finished item

        An item counts as finished once the next one is requested, so a state saved right
        after reading an item resumes at that item. With `verify_state` on, passing a copy
        as `state` raises `StateMismatchError` when the position moved after the state was
        created, loaded or saved. Reading, `options.start` and a byte or chunk
        `options.skip` move the position, so pass the `State` that `save()` returned instead
        """

@final
class State:
    """
    Saved progress of an iterator over one file

    Saving an iterator or loading from the registry returns one, and passing it back as the
    `state` of a new iterator resumes from it
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str:
        """
        Returns the class name with the name, file, position and timestamps
        """
    @property
    def file(self, /) -> FileMetadata:
        """
        Metadata of the tracked file, recorded when the state was created or last resynced
        """
    @property
    def name(self, /) -> str:
        """
        Name the state is saved under
        """
    def percent(self, /) -> float:
        """
        Returns `position` as a share of the recorded file size, from 0.0 to 100.0

        It returns 0.0 for an empty file
        """
    @property
    def position(self, /) -> int:
        """
        Byte offset just past the last finished item, where a resumed iterator starts
        """
    @property
    def timestamps(self, /) -> Timestamps:
        """
        Times the state was created and last saved or resynced
        """

class StateConflictError(StateError):
    """
    Another writer saved the state since this iterator loaded or last saved it

    The check starts once this iterator holds the saved state: an iterator that started fresh,
    or from a `State` other than the saved one, overwrites it on its first save. Open a new
    iterator to continue
    """
    def __new__(cls, message: str = ..., name: str = ..., /) -> StateConflictError:
        """
        Creates the error from a message and the state name, both optional and positional
        """

class StateCorruptError(StateError):
    """
    The state file does not parse, was edited outside preader, or belongs to another name

    Delete it with `reader.states.delete(error.name)` to start over
    """
    def __new__(cls, message: str = ..., name: str = ..., /) -> StateCorruptError:
        """
        Creates the error from a message and the state name, both optional and positional
        """

@disjoint_base
class StateError(Exception):
    """
    Base of every preader state error; `name` is the state it concerns

    It is raised itself when a part of the state name is a symlink or a case or Unicode alias
    """
    def __new__(cls, message: str = ..., name: str = ..., /) -> StateError:
        """
        Creates the error from a message and the state name, both optional and positional
        """
    def __str__(self, /) -> str:
        """
        Returns the error message
        """
    @property
    def name(self, /) -> str:
        """
        Name of the state the error concerns
        """

@final
class StateIterator:
    """
    Iterator over the names of the states saved in a state directory

    The `names` and `search` methods of [`StateRegistry`] return it. Each name uses `/` as
    separator, the order is unspecified, and a part of the directory that cannot be read gives
    an error
    """
    def __iter__(self, /) -> StateIterator:
        """
        Returns the iterator itself
        """
    def __next__(self, /) -> str:
        """
        Returns the next saved state name, in no particular order

        Raises `OSError` when part of the state directory cannot be read
        """
    def __repr__(self, /) -> str:
        """
        Returns the class name with the state directory and the pattern
        """
    @property
    def pattern(self, /) -> str | None:
        """
        Regular expression names must match, or `None` when every name is listed
        """
    @property
    def state_dir(self, /) -> Path:
        """
        State directory whose names this iterator lists
        """

class StateMismatchError(StateError):
    """
    The file is no longer the tracked one, or a passed `State` has unsaved progress

    Start over under a new state, or pass the `State` that `save()` returned. It is also the
    base of `StateOutdatedError`
    """
    def __new__(cls, message: str = ..., name: str = ..., /) -> StateMismatchError:
        """
        Creates the error from a message and the state name, both optional and positional
        """

class StateOutdatedError(StateMismatchError):
    """
    The tracked file moved, disappeared or changed

    Accept the change of a saved state with `reader.states.resync(error.name, file)`
    """
    def __new__(cls, message: str = ..., name: str = ..., /) -> StateOutdatedError:
        """
        Creates the error from a message and the state name, both optional and positional
        """

@final
class StateRegistry:
    """
    Registry of the states saved in one state directory

    Every reader exposes one as `states`. Each state is a JSON file named `<name>.state.json`
    under the state directory, and a `/` in a name becomes a subdirectory
    """
    def __contains__(self, name: str, /) -> bool:
        """
        Returns whether a state is saved under `name`, like `exists(name)`
        """
    def __delitem__(self, name: str, /) -> None:
        """
        Deletes the state saved under `name`, like `delete(name)`
        """
    def __getitem__(self, name: str, /) -> State:
        """
        Returns the state saved under `name`, like `load(name)`
        """
    def __iter__(self, /) -> StateIterator:
        """
        Returns an iterator over the saved state names, like `names()`
        """
    def __len__(self, /) -> int:
        """
        Returns the number of saved states, without reading them

        Raises `OSError` when the state directory cannot be listed
        """
    def __repr__(self, /) -> str:
        """
        Returns the class name with the state directory
        """
    def all(self, /) -> list[State]:
        """
        Returns every saved state as a list

        The first state that fails to load aborts the call and raises the same errors as `load()`.
        Raises `OSError` when the state directory path is not a directory
        """
    def clear(self, /) -> None:
        """
        Deletes every saved state, including corrupt ones

        It stops at the first failure, so some states may already be deleted when it raises
        `OSError`. An iterator that still holds a state writes it again on its next save, manual or
        autosave
        """
    def delete(self, /, name: str) -> None:
        """
        Deletes the state file saved under `name` without reading it

        An iterator that still holds the state writes it again on its next save, manual or
        autosave. Raises `KeyError` when nothing is saved under `name`, `ValueError` for an invalid
        name, `StateError` for an aliased name, and `OSError` when the file cannot be removed
        """
    def exists(self, /, name: str) -> bool:
        """
        Returns whether a state file is saved under `name`, without reading or checking it

        An invalid or aliased name returns `False` instead of raising
        """
    def find(self, /, name: str) -> State | None:
        """
        Returns the state saved under `name`, or `None` when it cannot be loaded

        Every error `load()` would raise becomes `None`, not only a missing state
        """
    def load(self, /, name: str) -> State:
        """
        Returns the state saved under `name`, checked against its file when `verify_state` is on

        Raises `KeyError` when nothing is saved under `name`, `ValueError` for an invalid name,
        `StateError` for an aliased name, `StateCorruptError` when the state file does not parse
        or belongs to another name, and `OSError` when a file cannot be read. With `verify_state`
        on, it also raises `StateCorruptError` for a state file edited outside preader,
        `StateOutdatedError` when the tracked file is missing or changed in size or mtime, and
        `StateMismatchError` when it is another file
        """
    def names(self, /) -> StateIterator:
        """
        Returns an iterator over the names of every saved state, in no particular order

        Names use `/` between directories on every platform, and a missing state directory lists
        nothing. Entries that are not valid state names, such as temporary files and symlinks, are
        skipped. Raises `OSError` when the state directory path is not a directory
        """
    def path(self, /, name: str) -> Path:
        """
        Returns the path of the state file for `name`, whether or not it exists

        Raises `ValueError` for an invalid name and `StateError` when a part of the name is a
        symlink or a case or Unicode alias
        """
    def resync(self, /, name: str, path: str | PathLike[str]) -> State:
        """
        Accepts a moved or changed file for the state saved under `name`, saves it and returns it

        Use it after a `StateOutdatedError`, as `reader.states.resync(error.name, file)`. The state
        keeps its name and its position, clamped to the new file size. It records the canonical
        `path`, so after a move an auto-named state resumes only when its name is passed as `state`.
        An iterator still holding the old state raises `StateConflictError` on its next save.
        Raises `KeyError`, `ValueError`, `StateError` and `StateCorruptError` as `load()` does for
        the saved state, `StateMismatchError` when `verify_state` finds that `path` holds another
        file, and `OSError` when `path` cannot be read or the state cannot be written
        """
    def search(self, /, pattern: str) -> StateIterator:
        """
        Returns an iterator over the saved state names that match the regular expression `pattern`

        The match is unanchored, so `pattern` may match any part of a name; anchor it with `^` and
        `$` to match whole names. The syntax is that of the Rust `regex` crate, without lookaround
        or backreferences. Raises `ValueError` for an invalid pattern and `OSError` when the state
        directory path is not a directory
        """
    @property
    def state_dir(self, /) -> Path:
        """
        Directory that holds the state files
        """

@final
class Timestamps:
    """
    Creation and last update times of a state, in whole UTC seconds

    `updated_at` changes only when the state is saved or resynced, not while an iterator advances
    """
    def __eq__(self, value: object, /) -> bool: ...
    def __ne__(self, value: object, /) -> bool: ...
    def __repr__(self, /) -> str:
        """
        Returns the class name with both timestamps
        """
    @property
    def created_at(self, /) -> datetime:
        """
        When the state was first created
        """
    @property
    def updated_at(self, /) -> datetime:
        """
        When the state was last saved or resynced
        """
