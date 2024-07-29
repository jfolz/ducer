from __future__ import annotations

from typing import final, SupportsInt, Tuple, SupportsBytes, Iterable, Iterator
from pathlib import Path


@final
class Buffer:
    def __buffer__(self, flags): ...

    def __release_buffer__(self, buffer): ...


@final
class Op:
    Avg = Op
    First = Op
    Last = Op
    Max = Op
    Median = Op
    Mid = Op
    Min = Op


@final
class Automaton:
    """
    Automata can be used to efficiently apply complex search patterns
    to the keys of maps and sets.
    Use one of the classmethods `never`, `always`, `str`,
    or `subsequence` to create a new automaton.
    Add more complex behvaior on top with `starts_with`, `complement`,
    `intersection`, or `union`.
    E.g., an automaton that mathes keys that start with `b"foo"` or `b"bar"`:

    ```Python
    a_foo = Automaton.str(b"foo")
    a_bar = Automaton.str(b"bar")
    a_foobar = a_foo.union(a_bar).starts_with()
    ```
    """

    @classmethod
    def always(cls) -> Automaton:
        """
        Create a new `Automaton` that always matches.
        """
        ...

    @classmethod
    def never(cls) -> Automaton:
        """
        Create a new `Automaton` that never matches.
        """
        ...

    @classmethod
    def str(cls, str: bytes) -> Automaton:
        """
        Create a new `Automaton` that matches `str` exactly.
        """
        ...

    @classmethod
    def subsequence(cls, str: bytes) -> Automaton:
        """
        Create a new `Automaton` that subsequences matches str.
        E.g., b"bd" matches the key b"abcde".
        """
        ...

    def complement(self) -> Automaton:
        """

        """
        ...

    def starts_with(self) -> Automaton:
        """

        """
        ...

    def intersection(self, other: Automaton) -> Automaton:
        """

        """
        ...

    def union(self, other: Automaton) -> Automaton:
        """

        """
        ...


@final
class Map:
    @classmethod
    def build(cls, path: str | Path, iterable: Iterable[Tuple[SupportsBytes, SupportsInt]]) -> Buffer | None: ...

    def copy(self) -> Map: ...

    def __init__(self, data): ...

    def __len__(self): ...

    def __iter__(self) -> Iterator[bytes]: ...

    def __getitem__(self, key) -> int: ...

    def __eq__(self, other) -> bool: ...

    def get(self, key, default=None) -> int | None: ...

    def keys(self) -> Iterator[bytes]: ...

    def values(self) -> Iterator[int]: ...

    def items(self) -> Iterator[Tuple[bytes, int]]: ...

    def range(self, ge: bytes | None = None, gt: bytes | None = None, le: bytes | None = None, lt: bytes | None = None) -> Iterator[Tuple[bytes, int]]: ...

    def starts_with(self, str: bytes, ge: bytes | None = None, gt: bytes | None = None, le: bytes | None = None, lt: bytes | None = None) -> Iterator[Tuple[bytes, int]]: ...

    def subsequence(self, str: bytes, ge: bytes | None = None, gt: bytes | None = None, le: bytes | None = None, lt: bytes | None = None) -> Iterator[Tuple[bytes, int]]: ...

    def search(self, automaton: Automaton, ge: bytes | None = None, gt: bytes | None = None, le: bytes | None = None, lt: bytes | None = None) -> Iterator[Tuple[bytes, int]]: ...

    def difference(self, path: str | Path, *others: Map, select=Op.Last) -> Buffer | None: ...

    def intersection(self, path: str | Path, *others: Map, select=Op.Last) -> Buffer | None: ...

    def symmetric_difference(self, path: str | Path, *others: Map, select=Op.Last) -> Buffer | None: ...

    def union(self, path: str | Path, *others: Map, select=Op.Last) -> Buffer | None: ...


@final
class Set:
    @classmethod
    def build(cls, path: str | Path, iterable: Iterable[SupportsBytes]) -> Buffer | None: ...

    def copy(self) -> Set: ...

    def __init__(self, data): ...

    def __len__(self): ...

    def __iter__(self) -> Iterator[bytes]: ...

    def __eq__(self, other) -> bool: ...

    def __gt__(self, other: Set) -> bool: ...

    def __ge__(self, other: Set) -> bool: ...

    def __lt__(self, other: Set) -> bool: ...

    def __le__(self, other: Set) -> bool: ...

    def isdisjoint(self, other: Set) -> bool: ...

    def issubset(self, other: Set) -> bool: ...

    def issuperset(self, other: Set) -> bool: ...

    def keys(self) -> Iterator[bytes]: ...

    def range(self, ge: bytes | None = None, gt: bytes | None = None, le: bytes | None = None, lt: bytes | None = None) -> Iterator[bytes]: ...

    def starts_with(self, str: bytes, ge: bytes | None = None, gt: bytes | None = None, le: bytes | None = None, lt: bytes | None = None) -> Iterator[bytes]: ...

    def subsequence(self, str: bytes, ge: bytes | None = None, gt: bytes | None = None, le: bytes | None = None, lt: bytes | None = None) -> Iterator[bytes]: ...

    def search(self, automaton: Automaton, ge: bytes | None = None, gt: bytes | None = None, le: bytes | None = None, lt: bytes | None = None) -> Iterator[bytes]: ...

    def difference(self, path: str | Path, *others: Set) -> Buffer | None: ...

    def intersection(self, path: str | Path, *others: Set) -> Buffer | None: ...

    def symmetric_difference(self, path: str | Path, *others: Set) -> Buffer | None: ...

    def union(self, path: str | Path, *others: Set) -> Buffer | None: ...
