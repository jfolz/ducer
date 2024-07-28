API reference
====================

This module provides classes and methods for working with finite-state transducers, enabling efficient operations on maps and sets.



.. class:: Buffer

   Represents a buffer that can be converted to bytes or accessed in a buffer interface.

   .. method:: __bytes__() -> bytes

      Converts the buffer to a byte representation.

   .. method:: __getbuffer__()

      Provides access to the buffer's data.

   .. method:: __releasebuffer__()

      Releases the buffer’s data.



.. class:: Op

   Contains strategies for resolving conflicts during set operations on maps.

   .. attribute:: Avg

   .. attribute:: First

   .. attribute:: Last

   .. attribute:: Max

   .. attribute:: Median

   .. attribute:: Mid

   .. attribute:: Min



.. class:: Automaton

   Automata for efficiently applying complex search patterns to keys of maps and sets.
   Use class methods like `never`, `always`, `str`, or `subsequence` to create a new automaton. Modify behavior with methods like `starts_with`, `complement`, `intersection`, or `union`.

   Example:

   .. code-block:: python

      a_foo = Automaton.str(b"foo")
      a_bar = Automaton.str(b"bar")
      a_foobar = a_foo.union(a_bar).starts_with()

   .. method:: always() -> Automaton

      Create a new `Automaton` that always matches.

   .. method:: never() -> Automaton

      Create a new `Automaton` that never matches.

   .. method:: str(str: bytes) -> Automaton

      Create a new `Automaton` that matches `str` exactly.

   .. method:: subsequence(str: bytes) -> Automaton

      Create a new `Automaton` that matches any subsequence of `str`. For example, `"bd"` matches the key `"abcde"`.

   .. method:: complement() -> Automaton

      Modify this automaton to match any key that would previously not match. Returns `self` for chaining with other methods.

   .. method:: starts_with() -> Automaton

      Modify this automaton to match any key that starts with a prefix that would previously match. Returns `self` for chaining with other methods.

   .. method:: intersection(other: Automaton) -> Automaton

      Modify this automaton to match any key that both `self` and `other` match. `other` must be an instance of `Automaton`. Returns `self` for chaining with other methods.

   .. method:: union(other: Automaton) -> Automaton

      Modify this automaton to match any key that either `self` or `other` matches. `other` must be an instance of `Automaton`. Returns `self` for chaining with other methods.



.. class:: Map

   An immutable map with `bytes` keys and non-negative integer values, based on finite-state transducers. It typically uses less memory compared to the built-in `dict` and can be streamed from a file.

   .. method:: build(path: str | Path, iterable: Iterable[Tuple[SupportsBytes, SupportsInt]]) -> Buffer | None

      Build a `Map` from an iterable of `(key: bytes, value: int)` and write it to the given path. If `path` is `":memory:"`, returns a `Buffer` containing the map data. `path` can be of type `str` or `pathlib.Path`.

   .. method:: __init__(data)

      Initialize the `Map` with the given data.

   .. method:: __len__() -> int

      Return the number of items in the map.

   .. method:: __iter__() -> Iterator[bytes]

      Return an iterator over the keys in the map.

   .. method:: __getitem__(key) -> int

      Return the value for the given `key`.

   .. method:: __eq__(other) -> bool

      Check if `self` is equal to `other`.

   .. method:: get(key, default=None) -> int | None

      Return the value for the given `key` if present, or `default` otherwise.

   .. method:: keys() -> Iterator[bytes]

      Iterate over all keys.

   .. method:: values() -> Iterator[int]

      Iterate over all values.

   .. method:: items() -> Iterator[Tuple[bytes, int]]

      Iterate over all key-value items.

   .. method:: range(ge=None, gt=None, le=None, lt=None) -> Iterator[Tuple[bytes, int]]

      Iterate over all key-value items with optional range limits for the key: `ge` (greater than or equal), `gt` (greater than), `le` (less than or equal), and `lt` (less than). If no limits are given, this is equivalent to `iter(self)`.

   .. method:: starts_with(str: bytes, ge=None, gt=None, le=None, lt=None) -> Iterator[Tuple[bytes, int]]

      Iterate over all key-value items whose key starts with `str`. Optionally apply range limits: `ge` (greater than or equal), `gt` (greater than), `le` (less than or equal), and `lt` (less than).

   .. method:: subsequence(str: bytes, ge=None, gt=None, le=None, lt=None) -> Iterator[Tuple[bytes, int]]

      Iterate over all key-value items whose key contains the subsequence `str`. Keys do not need to contain the subsequence consecutively. Optionally apply range limits: `ge` (greater than or equal), `gt` (greater than), `le` (less than or equal), and `lt` (less than).

   .. method:: search(automaton: Automaton, ge=None, gt=None, le=None, lt=None) -> Iterator[Tuple[bytes, int]]

      Iterate over all key-value items whose key matches the given `Automaton`. Optionally apply range limits: `ge` (greater than or equal), `gt` (greater than), `le` (less than or equal), and `lt` (less than).

   .. method:: difference(*others, select=Op.Last) -> Buffer | None

      Build a new map that is the difference between `self` and all `others`, meaning the resulting map will contain all keys that are in `self`, but not in `others`. `others` must be instances of `Map`. `select` specifies how conflicts are resolved if keys are present more than once. If `path` is `":memory:"`, returns a `Buffer` containing the map data instead of writing to `path`. `path` can be `str` or `pathlib.Path`.

   .. method:: intersection(*others, select=Op.Last) -> Buffer | None

      Build a new map that is the intersection of `self` and `others`. `others` must be instances of `Map`. `select` specifies how conflicts are resolved if keys are present more than once. If `path` is `":memory:"`, returns a `Buffer` containing the map data instead of writing to `path`. `path` can be `str` or `pathlib.Path`.

   .. method:: symmetric_difference(*others, select=Op.Last) -> Buffer | None

      Build a new map that is the symmetric difference between `self` and `others`, meaning the resulting map will contain all keys that occur an odd number of times. `others` must be instances of `Map`. `select` specifies how conflicts are resolved if keys are present more than once. If `path` is `":memory:"`, returns a `Buffer` containing the map data instead of writing to `path`. `path` can be `str` or `pathlib.Path`.

   .. method:: union(*others, select=Op.Last) -> Buffer | None

      Build a new map that is the union of `self` and `others`. `others` must be instances of `Map`. `select` specifies how conflicts are resolved if keys are present more than once. If `path` is `":memory:"`, returns a `Buffer` containing the map data instead of writing to `path`. `path` can be `str` or `pathlib.Path`.



.. class:: Set

   An immutable set of `bytes` keys, based on finite-state transducers. It typically uses less memory compared to the built-in `set` and can be streamed from a file.

   .. method:: build(path: str | Path, iterable: Iterable[SupportsBytes]) -> Buffer | None

      Build a `Set` from an iterable of `bytes` and write it to the given path. If `path` is `":memory:"`, returns a `Buffer` containing the set data. `path` can be of type `str` or `pathlib.Path`.

   .. method:: __init__(data)

      Initialize the `Set` with the given data.

   .. method:: __len__() -> int

      Return the number of items in the set.

   .. method:: __iter__() -> Iterator[bytes]

      Return an iterator over the keys in the set.

   .. method:: __eq__(other) -> bool

      Check if `self` is equal to `other`.

   .. method:: __gt__(other: Set) -> bool

      Check if `self` is greater than `other`.

   .. method:: __ge__(other: Set) -> bool

      Check if `self` is greater than or equal to `other`.

   .. method:: __lt__(other: Set) -> bool

      Check if `self` is less than `other`.

   .. method:: __le__(other: Set) -> bool

      Check if `self` is less than or equal to `other`.

   .. method:: isdisjoint(other: Set) -> bool

      Return `True` if the set has no elements in common with `other`. Sets are disjoint if their intersection is the empty set.

   .. method:: issubset(other: Set) -> bool

      Test whether every element in `self` is in `other`.

   .. method:: issuperset(other: Set) -> bool

      Test whether every element in `other` is in `self`.

   .. method:: keys() -> Iterator[bytes]

      Return an iterator over the keys in the set. This is equivalent to `iter(self)`.

   .. method:: range(ge=None, gt=None, le=None, lt=None) -> Iterator[bytes]

      Iterate over all keys with optional range limits: `ge` (greater than or equal), `gt` (greater than), `le` (less than or equal), and `lt` (less than). If no limits are given, this is equivalent to `iter(self)`.

   .. method:: starts_with(str: bytes, ge=None, gt=None, le=None, lt=None) -> Iterator[bytes]

      Iterate over all keys that start with `str`. Optionally apply range limits: `ge` (greater than or equal), `gt` (greater than), `le` (less than or equal), and `lt` (less than).

   .. method:: subsequence(str: bytes, ge=None, gt=None, le=None, lt=None) -> Iterator[bytes]

      Iterate over all keys that contain the subsequence `str`. Keys do not need to contain the subsequence consecutively. Optionally apply range limits: `ge` (greater than or equal), `gt` (greater than), `le` (less than or equal), and `lt` (less than).

   .. method:: search(automaton: Automaton, ge=None, gt=None, le=None, lt=None) -> Iterator[bytes]

      Iterate over all keys that match the given `Automaton`. Optionally apply range limits: `ge` (greater than or equal), `gt` (greater than), `le` (less than or equal), and `lt` (less than).

   .. method:: difference(*others, select=Op.Last) -> Buffer | None

      Build a new set that is the difference between `self` and all `others`, meaning the resulting set will contain all keys that are in `self`, but not in `others`. `others` must be instances of `Set`. `select` specifies how conflicts are resolved if keys are present more than once. If `path` is `":memory:"`, returns a `Buffer` containing the set data instead of writing to `path`. `path` can be `str` or `pathlib.Path`.

   .. method:: intersection(*others, select=Op.Last) -> Buffer | None

      Build a new set that is the intersection of `self` and `others`. `others` must be instances of :class:`Set`. `select` specifies how conflicts are resolved if keys are present more than once. If `path` is `":memory:"`, returns a `Buffer` containing the set data instead of writing to `path`. `path` can be `str` or `pathlib.Path`.

   .. method:: symmetric_difference(*others, select=Op.Last) -> Buffer | None

      Build a new set that is the symmetric difference between `self` and `others`, meaning the resulting set will contain all keys that occur an odd number of times. `others` must be instances of `Set`. `select` specifies how conflicts are resolved if keys are present more than once. If `path` is `":memory:"`, returns a `Buffer` containing the set data instead of writing to `path`. `path` can be `str` or `pathlib.Path`.

   .. method:: union(*others, select=Op.Last) -> Buffer | None

      Build a new set that is the union of `self` and `others`. `others` must be instances of `Set`. `select` specifies how conflicts are resolved if keys are present more than once. If `path` is `":memory:"`, returns a `Buffer` containing the set data instead of writing to `path`. `path` can be `str` or `pathlib.Path`.
