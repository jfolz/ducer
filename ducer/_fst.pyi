from __future__ import annotations

from typing import final, SupportsInt, Tuple, SupportsBytes, Iterable, Iterator
from pathlib import Path


@final
class Buffer:
    """
    A read-only buffer returned by Map.build and Set.build
    when path is ":memory:".
    Use to create new Map or Set instances, or write to file:

        from ducer import Set
        buf = Set.build([b"a", b"b"], ":memory:")
        s = Set(buf)
        for k in s:
            print(k)
        with open("my.set", "wb") as f:
            f.write(buf)
    """

    def __buffer__(self, flags): ...

    def __release_buffer__(self, buffer):
        """
        Release the buffer object that exposes the underlying memory of the object.
        """
        ...


@final
class Op:
    """
    Conflict resolution strategies for set operations on maps.
    """

    Avg = Op()
    """
    Select average, i.e., sum(values) // len.
    """

    First = Op()
    """
    Select first value.
    """

    Last = Op()
    """
    Select last value.
    """

    Max = Op()
    """
    Select maximum.
    """

    Median = Op()
    """
    Select median, i.e., with values = sorted(values) and mid = len // 2,
    select values[mid] for odd length,
    and (values[mid-1] + values[mid]) // 2 for even length.
    """

    Mid = Op()
    """

    Select middle value, i.e., values[len // 2].
    """

    Min = Op()
    """
    Select minimum.
    """


@final
class Automaton:
    """
    Automata can be used to efficiently apply complex search patterns
    to the keys of maps and sets.
    Use one of the classmethods never, always, str,
    or subsequence to create a new automaton.
    Add more complex behavior on top with starts_with, complement,
    intersection, or union.
    E.g., an automaton that matches keys that start with b"foo" or b"bar":

        a_foo = Automaton.str(b"foo")
        a_bar = Automaton.str(b"bar")
        a_foobar = a_foo.union(a_bar).starts_with()
    """

    @classmethod
    def always(cls) -> Automaton:
        """
        Create a new Automaton that always matches.
        """
        ...

    @classmethod
    def never(cls) -> Automaton:
        """
        Create a new Automaton that never matches.
        """
        ...

    @classmethod
    def str(cls, str: bytes) -> Automaton:
        """
        Create a new Automaton that matches str exactly.
        """
        ...

    @classmethod
    def subsequence(cls, str: bytes) -> Automaton:
        """
        Create a new Automaton that subsequences matches str.
        E.g., b"bd" matches the key b"abcde".
        """
        ...

    def complement(self) -> Automaton:
        """
        Modify this automaton to match any key that would previously not match.
        Returns self to allow chaining with other methods.
        """
        ...

    def starts_with(self) -> Automaton:
        """
        Modify this automaton to match any key that starts with a prefix that previously matched,
        e.g., if self matched b"abc", it will now match b"abcde".
        Returns self to allow chaining with other methods.
        """
        ...

    def intersection(self, other: Automaton) -> Automaton:
        """
        Modify this automaton to match any key that both self and other matches.
        other must be an instance of Automaton.
        Returns self to allow chaining with other methods.
        """
        ...

    def union(self, other: Automaton) -> Automaton:
        """
        Modify this automaton to match any key that either self or other matches.
        other must be an instance of Automaton.
        Returns self to allow chaining with other methods.
        """
        ...


@final
class Map:
    """
    An immutable map of bytes keys and non-negative integers, based on finite-state-transducers.
    Typically uses a fraction of the memory as the builtin dict and can be streamed from a file.

    data can be any object that supports the buffer protocol,
    e.g., Buffer, bytes, memoryview, mmap, etc.
    Use Map.build to create suitable data.

    Important: data needs to be contiguous.

    To the extent that it's feasible, ducer maps are intended to be direct replacements for the builtin dict.
    For m, o: Map and k: bytes, the following works as intended:

        k in m
        m == o
        m[k]
        m.get(k)
        m.get(k, 42)
        len(m)
        for k in m:
            pass
        for k in m.keys():
            pass
        for v in m.values():
            pass
        for k, v in m.items():
            pass

    Since maps are immutable, the following are not implemented:

    - clear
    - fromkeys
    - pop
    - popitem
    - setdefault
    - update, |=

    Further, the |, &, -, ^ operators are also not implemented,
    since it is not possible to specify the storage path.
    Use Map.union, Map.intersection, Map.difference, and Map.symmetric_difference instead.
    """

    def __init__(self, data: SupportsBytes):
        """
        Create a Set from the given data.
        data can be any object that supports the buffer protocol,
        e.g., bytes, memoryview, mmap, etc.
        Important: data needs to be contiguous.
        """
        ...

    @classmethod
    def build(cls, path: str | Path, iterable: Iterable[Tuple[SupportsBytes, SupportsInt]]) -> Buffer | None:
        """
        Build a map from an iterable of items (key: bytes, value: int)
        and write it to the given path.
        If path is ":memory:", returns a Buffer containing the map data.
        path can be str or Path.

        Hint:
            Items can really be any sequence of length 2, but building from tuple is fastest.
            However, avoid converting items in Python for best performance.
            Ideally, create tuples directly, e.g., if using msgpack,
            set use_list=False for msgpack.unpackb or msgpack.Unpacker.

        """
        ...

    def copy(self) -> Map:
        """
        Since maps are immutable, returns self.
        """
        ...

    def __len__(self):
        """
        Returns number of items in this map.
        """
        ...

    def __contains__(self, key: bytes) -> bool:
        """
        Returns whether this map contains key.
        """
        ...

    def __iter__(self) -> Iterator[bytes]:
        """
        Implement iter(self).
        Like the builtin dict, only keys are returned.
        """
        ...

    def __getitem__(self, key) -> int:
        """
        Implement self[key].
        """
        ...

    def __eq__(self, other) -> bool:
        """
        Returns whether this map equals other.
        Other must be Map.
        """
        ...

    def get(self, key, default=None) -> int | None:
        """
        Returns the given key if present, default otherwise.
        """
        ...

    def keys(self) -> Iterator[bytes]:
        """
        Iterate over all keys.
        """
        ...

    def values(self) -> Iterator[int]:
        """
        Iterate over all values.
        """
        ...

    def items(self) -> Iterator[Tuple[bytes, int]]:
        """
        Iterate over all key-value items.
        """
        ...

    def range(self, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[Tuple[bytes, int]]:
        """
        Iterate over all key-value items with optional range limits for the key
        ge (greater than or equal),
        gt (greater than),
        le (less than or equal),
        and lt (less than).
        If no limits are given this is equivalent to iter(self).
        """
        ...

    def starts_with(self, str: bytes, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[Tuple[bytes, int]]:
        """
        Iterate over all key-value items whose key starts with str.
        Optionally apply range limits
        ge (greater than or equal),
        gt (greater than),
        le (less than or equal),
        and lt (less than).
        """
        ...

    def subsequence(self, str: bytes, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[Tuple[bytes, int]]:
        """
        Iterate over all key-value items whose key contain the subsequence str.
        Keys don't need to contain the subsequence consecutively,
        e.g., b"bd" will match the key b"abcde".
        Optionally apply range limits
        ge (greater than or equal),
        gt (greater than),
        le (less than or equal),
        and lt (less than).
        """
        ...

    def search(self, automaton: Automaton, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[Tuple[bytes, int]]:
        """
        Iterate over all key-value items whose key matches the given Automaton.
        Optionally apply range limits
        ge (greater than or equal),
        gt (greater than),
        le (less than or equal),
        and lt (less than).
        """
        ...

    def difference(self, path: str | Path, *others: Map, select: Op=Op.Last) -> Buffer | None:
        """
        Build a new map that is the difference between self and all others,
        meaning the resulting map will contain all keys that are in self,
        but not in others.
        others must be instances of Map.
        select specifies how conflicts are resolved if keys are
        present more than once.
        If path is ":memory:", returns a Buffer containing the map data
        instead of writing to path.
        path can be str or Path.
        """
        ...

    def intersection(self, path: str | Path, *others: Map, select: Op=Op.Last) -> Buffer | None:
        """
        Build a new map that is the intersection of self and others.
        others must be instances of Map.
        select specifies how conflicts are resolved if keys are
        present more than once.
        If path is ":memory:", returns a Buffer containing the map data
        instead of writing to path.
        path can be str or Path.
        """
        ...

    def symmetric_difference(self, path: str | Path, *others: Map, select: Op=Op.Last) -> Buffer | None:
        """
        Build a new map that is the symmetric difference between self and others.
        The resulting map will contain all keys that appear an odd number of times, i.e.,
        if only one other map is given, it will contain all keys that are in either
        self or others, but not in both.
        others must be instances of Map.
        select specifies how conflicts are resolved if keys are
        present more than once.
        If path is ":memory:", returns a Buffer containing the map data
        instead of writing to path.
        path can be str or Path.
        """
        ...

    def union(self, path: str | Path, *others: Map, select: Op=Op.Last) -> Buffer | None:
        """
        Build a new map that is the union of self and others.
        others must be instances of Map.
        select specifies how conflicts are resolved if keys are
        present more than once.
        If path is ":memory:", returns a Buffer containing the map data
        instead of writing to path.
        path can be str or Path.
        """
        ...


@final
class Set:
    """
    An immutable set of bytes keys, based on finite-state-transducers.
    Typically uses a fraction of the memory as the builtin set and can be streamed from a file.

    data can be any object that supports the buffer protocol,
    e.g., Buffer, bytes, memoryview, mmap, etc.
    Use Map.build to create suitable data.

    Important: data needs to be contiguous.

    To the extent that it's feasible, ducer sets are intended to be direct replacements for the builtin set.
    For s, o: Set, and k: bytes, the following works as intended:

        k in s
        s == o
        len(s)
        for k in s:
            pass
        s.isdisjoint(o)
        s.issubset(o)
        s <= o  # subset
        s < o  # proper subset
        s.issuperset(o)
        s >= o  # superset
        s > o  # proper superset

    Since sets are immutable, the following are **not implemented**:

    - add
    - clear
    - difference_update, -=
    - discard
    - intersection_update, &=
    - pop
    - remove
    - symmetric_difference_update, ^=
    - update, |=

    Further, the |, &, -, ^ operators are also not implemented,
    since it is not possible to specify the storage path.
    Use Set.union, Set.intersection, Set.difference, and Set.symmetric_difference instead.
    """

    def __init__(self, data: SupportsBytes):
        """
        Create a Set from the given data.
        data can be any object that supports the buffer protocol,
        e.g., bytes, memoryview, mmap, etc.
        Important: data needs to be contiguous.
        """
        ...

    @classmethod
    def build(cls, path: str | Path, iterable: Iterable[SupportsBytes]) -> Buffer | None:
        """
        Build a Set from an iterable of bytes
        and write it to the given path.
        If path is ":memory:", returns a Buffer containing the set data.
        path can be str or Path.
        """
        ...

    def copy(self) -> Set:
        """
        Since sets are immutable, returns self.
        """
        ...

    def __len__(self):
        """
        Returns number of keys in this set.
        """
        ...

    def __contains__(self, key: bytes) -> bool:
        """
        Returns whether key is in this set.
        """
        ...

    def __iter__(self) -> Iterator[bytes]:
        """
        Implement iter(self).
        """
        ...

    def __eq__(self, other) -> bool:
        """
        Returns this set equals other.
        other must be Set.
        """
        ...

    def __gt__(self, other: Set) -> bool:
        """
        Returns whether this set is a proper superset of other.
        other must be Set.
        """
        ...

    def __ge__(self, other: Set) -> bool:
        """
        Returns whether this set is a superset of other.
        other must be Set.
        """
        ...

    def __lt__(self, other: Set) -> bool:
        """
        Returns whether this set is a proper subset of other.
        other must be Set.
        """
        ...

    def __le__(self, other: Set) -> bool:
        """
        Returns whether this set is a subset of other.
        other must be Set.
        """
        ...

    def isdisjoint(self, other: Set) -> bool:
        """
        Return True if the set has no elements in common with other.
        Sets are disjoint if and only if their intersection is the empty set.
        """
        ...

    def issubset(self, other: Set) -> bool:
        """
        Test whether every element in the set is in other.
        """
        ...

    def issuperset(self, other: Set) -> bool:
        """
        Test whether every element in other is in the set.
        """
        ...

    def keys(self) -> Iterator[bytes]:
        """
        Iterate over all keys.
        """
        ...

    def range(self, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[bytes]:
        """
        Iterate over all keys with optional range limits
        ge (greater than or equal),
        gt (greater than),
        le (less than or equal),
        and lt (less than).
        If no limits are given this is equivalent to iter(self).
        """
        ...

    def starts_with(self, str: bytes, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[bytes]:
        """
        Iterate over all keys that start with str.
        Optionally apply range limits
        ge (greater than or equal),
        gt (greater than),
        le (less than or equal),
        and lt (less than).
        """
        ...

    def subsequence(self, str: bytes, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[bytes]:
        """
        Iterate over all keys that contain the subsequence str.
        Keys don't need to contain the subsequence consecutively,
        e.g., b"bd" will match the key b"abcde".
        Optionally apply range limits
        ge (greater than or equal),
        gt (greater than),
        le (less than or equal),
        and lt (less than).
        """
        ...

    def search(self, automaton: Automaton, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[bytes]:
        """
        Iterate over all keys that match the given Automaton.
        Optionally apply range limits
        ge (greater than or equal),
        gt (greater than),
        le (less than or equal),
        and lt (less than).
        """
        ...

    def difference(self, path: str | Path, *others: Set) -> Buffer | None:
        """
        Build a new set that is the difference between self and all others,
        meaning the resulting set will contain all keys that are in self,
        but not in others.
        others must be instances of Set.
        If path is ":memory:", returns a Buffer containing the set data
        instead of writing to path.
        path can be str or Path.
        """
        ...

    def intersection(self, path: str | Path, *others: Set) -> Buffer | None:
        """
        Build a new set that is the intersection of self and others.
        others must be instances of Set.
        If path is ":memory:", returns a Buffer containing the set data
        instead of writing to path.
        path can be str or Path.
        """
        ...

    def symmetric_difference(self, path: str | Path, *others: Set) -> Buffer | None:
        """
        Build a new set that is the symmetric difference between self and others.
        The resulting set will contain all keys that appear an odd number of times, i.e.,
        if only one other set is given, it will contain all keys that are in either
        self or others, but not in both.
        others must be instances of Set.
        If path is ":memory:", returns a Buffer containing the set data
        instead of writing to path.
        path can be str or Path.
        """
        ...

    def union(self, path: str | Path, *others: Set) -> Buffer | None:
        """
        Build a new set that is the union of self and others.
        others must be instances of Set.
        If path is ":memory:", returns a Buffer containing the set data
        instead of writing to path.
        path can be str or Path.
        """
        ...
