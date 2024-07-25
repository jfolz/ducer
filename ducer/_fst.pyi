from typing import Collection, Sequence, SupportsInt, Tuple, SupportsBytes, Iterable, ByteString
from pathlib import Path
from typing_extensions import Iterator


class Buffer:
    def __bytes__(self): ...


class Op:
    Avg = Op
    First = Op
    Last = Op
    Max = Op
    Median = Op
    Mid = Op
    Min = Op


class Automaton:
    @classmethod
    def always(cls) -> Automaton: ...

    @classmethod
    def never(cls) -> Automaton: ...

    @classmethod
    def str(cls, str: str) -> Automaton: ...

    @classmethod
    def subsequence(cls, str: str) -> Automaton: ...

    def complement(self) -> Automaton: ...

    def starts_with(self) -> Automaton: ...

    def intersection(self, other: Automaton) -> Automaton: ...

    def union(self, other: Automaton) -> Automaton: ...


class Map:
    @classmethod
    def build(cls, iterable: Iterable[Tuple[SupportsBytes, SupportsInt]], path: str | Path) -> Buffer | None: ...

    def __init__(self, data): ...

    def __len__(self): ...

    def __iter__(self) -> Iterator[bytes]: ...

    def __getitem__(self, key) -> int: ...

    def get(self, key, default=None) -> int | None: ...

    def keys(self) -> Iterator[bytes]: ...

    def values(self) -> Iterator[int]: ...

    def items(self) -> Iterator[Tuple[bytes, int]]: ...

    def range(self, ge=None, gt=None, le=None, lt=None) -> Iterator[Tuple[bytes, int]]: ...

    def starts_with(self, str, ge=None, gt=None, le=None, lt=None) -> Iterator[Tuple[bytes, int]]: ...

    def subsequence(self, str, ge=None, gt=None, le=None, lt=None) -> Iterator[Tuple[bytes, int]]: ...

    def search(self, automaton: Automaton, ge=None, gt=None, le=None, lt=None) -> Iterator[Tuple[bytes, int]]: ...

    @classmethod
    def difference(cls, *maps, select=Op.Last) -> Buffer | None: ...

    @classmethod
    def intersection(cls, *maps, select=Op.Last) -> Buffer | None: ...

    @classmethod
    def symmetric_difference(cls, *maps, select=Op.Last) -> Buffer | None: ...

    @classmethod
    def union(cls, *maps, select=Op.Last) -> Buffer | None: ...

class Set:
    @classmethod
    def build(cls, iterable: Iterable[SupportsBytes], path: str | Path) -> Buffer | None: ...

    def __init__(self, data): ...

    def __len__(self): ...

    def __iter__(self) -> Iterator[bytes]: ...

    def isdisjoint(self, other: Set) -> bool: ...

    def issubset(self, other: Set) -> bool: ...

    def issuperset(self, other: Set) -> bool: ...

    def keys(self) -> Iterator[bytes]: ...

    def range(self, ge=None, gt=None, le=None, lt=None) -> Iterator[bytes]: ...

    def starts_with(self, str, ge=None, gt=None, le=None, lt=None) -> Iterator[bytes]: ...

    def subsequence(self, str, ge=None, gt=None, le=None, lt=None) -> Iterator[bytes]: ...

    def search(self, automaton: Automaton, ge=None, gt=None, le=None, lt=None) -> Iterator[bytes]: ...

    @classmethod
    def difference(cls, *maps, select=Op.Last) -> Buffer | None: ...

    @classmethod
    def intersection(cls, *maps, select=Op.Last) -> Buffer | None: ...

    @classmethod
    def symmetric_difference(cls, *maps, select=Op.Last) -> Buffer | None: ...

    @classmethod
    def union(cls, *maps, select=Op.Last) -> Buffer | None: ...
