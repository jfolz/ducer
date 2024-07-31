API reference
====================



.. class:: Automaton

    Automata can be used to efficiently apply complex search patterns to the keys of maps and sets.
    Use one of the classmethods :meth:`never`, :meth:`always`, :meth:`str`, or :meth:`subsequence` to create a new automaton.
    Add more complex behavior on top with :meth:`starts_with`, :meth:`complement`, :meth:`intersection`, or :meth:`union`.
    E.g., an automaton that matches keys that start with ``b"foo"`` or ``b"bar"``::

        a_foo = Automaton.str(b"foo")
        a_bar = Automaton.str(b"bar")
        a_foobar = a_foo.union(a_bar).starts_with()

    .. classmethod:: always(cls) -> Automaton

        Create a new ``Automaton`` that always matches.

    .. classmethod:: never(cls) -> Automaton

        Create a new ``Automaton`` that never matches.

    .. classmethod:: str(cls, str: bytes) -> Automaton

        Create a new ``Automaton`` that matches :class:`str` exactly.

    .. classmethod:: subsequence(cls, str: bytes) -> Automaton

        Create a new ``Automaton`` that subsequences matches :class:`str`.
        E.g., ``b"bd"`` matches the key ``b"abcde"``.

    .. method:: complement(self) -> Automaton

        Modify this automaton to match any key that would previously not match.
        Returns ``self`` to allow chaining with other methods.

    .. method:: starts_with(self) -> Automaton

        Modify this automaton to match any key that starts with a prefix that previously matched,
        e.g., if ``self`` matched ``b"abc"``, it will now match ``b"abcde"``.
        Returns ``self`` to allow chaining with other methods.

    .. method:: intersection(self, other: Automaton) -> Automaton

        Modify this automaton to match any key that both ``self`` and ``other`` matches.
        ``other`` must be an instance of ``Automaton``.
        Returns ``self`` to allow chaining with other methods.

    .. method:: union(self, other: Automaton) -> Automaton

        Modify this automaton to match any key that either ``self`` or ``other`` matches.
        ``other`` must be an instance of ``Automaton``.
        Returns ``self`` to allow chaining with other methods.



.. class:: Buffer

    A read-only buffer returned by :meth:`Map.build` and :meth:`Set.build` when :class:`~pathlib.Path` is ``":memory:"``.
    Use to create new :class:`Map` or :class:`Set` instances, or write to file::

        from ducer import Set
        buf = Set.build([b"a", b"b"], ":memory:")
        s = Set(buf)
        for k in s:
            print(k)
        with open("my.set", "wb") as f:
            f.write(buf)



.. class:: Map(data: SupportsBytes)

    An immutable map of bytes keys and non-negative integers, based on finite-state-transducers.
    Typically uses a fraction of the memory as the builtin :class:`dict` and can be streamed from a file.

    ``data`` can be any object that supports the buffer protocol,
    e.g., :class:`Buffer`, :class:`bytes`, :class:`memoryview`, :class:`~mmap.mmap`, etc.
    Use :meth:`Map.build` to create suitable ``data``.

    .. important:: ``data`` needs to be contiguous.

    To the extent that it's feasible, ducer maps are intended to be direct replacements for the builtin :class:`dict`.
    For ``m, o: Map`` and ``k: bytes``, the following works as intended::

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

    * ``clear``
    * ``fromkeys``
    * ``pop``
    * ``popitem``
    * ``setdefault``
    * ``update``, ``|=``

    Further, the ``|``, ``&``, ``-``, ``^`` operators are also not implemented,
    since it is not possible to specify the storage path.
    Use :meth:`Map.union`, :meth:`Map.intersection`, :meth:`Map.difference`, and :meth:`Map.symmetric_difference` instead.

    .. classmethod:: build(cls, path: str | ~pathlib.Path, iterable: Iterable[Tuple[SupportsBytes, SupportsInt]]) -> Buffer | None

        Build a map from an iterable of items ``(key: bytes, value: int)`` and write it to the given path.
        If ``path`` is ``":memory:"``, returns a :class:`Buffer` containing the map data.
        ``path`` can be :class:`str` or :class:`~pathlib.Path`.

        .. hint::
            Items can really be any sequence of length 2, but building from :class:`tuple` is fastest.
            However, avoid converting items in Python for best performance.
            Ideally, create tuples directly, e.g., if using msgpack,
            set ``use_list=False`` for :func:`msgpack.unpackb` or :class:`msgpack.Unpacker`.

    .. method:: copy(self) -> Map

        Since maps are immutable, returns self.

    .. method:: get(self, key, default=None) -> int | None

        Returns the given ``key`` if present, ``default`` otherwise.

    .. method:: keys(self) -> Iterator[bytes]

        Iterate over all keys.

    .. method:: values(self) -> Iterator[int]

        Iterate over all values.

    .. method:: items(self) -> Iterator[Tuple[bytes, int]]

        Iterate over all key-value items.

    .. method:: range(self, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[Tuple[bytes, int]]

        Iterate over all key-value items with optional range limits for the key
        ``ge`` (greater than or equal),
        ``gt`` (greater than),
        ``le`` (less than or equal),
        and ``lt`` (less than).
        If no limits are given this is equivalent to ``iter(self)``.

    .. method:: starts_with(self, str: bytes, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[Tuple[bytes, int]]

        Iterate over all key-value items whose key starts with :class:`str`.
        Optionally apply range limits
        ``ge`` (greater than or equal),
        ``gt`` (greater than),
        ``le`` (less than or equal),
        and ``lt`` (less than).

    .. method:: subsequence(self, str: bytes, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[Tuple[bytes, int]]

        Iterate over all key-value items whose key contain the subsequence :class:`str`.
        Keys don't need to contain the subsequence consecutively,
        e.g., ``b"bd"`` will match the key ``b"abcde"``.
        Optionally apply range limits
        ``ge`` (greater than or equal),
        ``gt`` (greater than),
        ``le`` (less than or equal),
        and ``lt`` (less than).

    .. method:: search(self, automaton: Automaton, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[Tuple[bytes, int]]

        Iterate over all key-value items whose key matches the given ``Automaton``.
        Optionally apply range limits
        ``ge`` (greater than or equal),
        ``gt`` (greater than),
        ``le`` (less than or equal),
        and ``lt`` (less than).

    .. method:: difference(self, path: str | ~pathlib.Path, *others: Map, select: Op=Op.Last) -> Buffer | None

        Build a new map that is the difference between ``self`` and all ``others``,
        meaning the resulting map will contain all keys that are in ``self``, but not in ``others``.
        ``others`` must be instances of :class:`Map`.
        ``select`` specifies how conflicts are resolved if keys are present more than once.
        If :class:`~pathlib.Path` is ``":memory:"``, returns a :class:`Buffer` containing the map data instead of writing to path.
        Path can be :class:`str` or :class:`~pathlib.Path`.

    .. method:: intersection(self, path: str | ~pathlib.Path, *others: Map, select: Op=Op.Last) -> Buffer | None

        Build a new map that is the intersection of ``self`` and ``others``.
        ``others`` must be instances of :class:`Map`.
        ``select`` specifies how conflicts are resolved if keys are present more than once.
        If :class:`~pathlib.Path` is ``":memory:"``, returns a :class:`Buffer` containing the map data instead of writing to path.
        :class:`~pathlib.Path` can be :class:`str` or :class:`~pathlib.Path`.

    .. method:: symmetric_difference(self, path: str | ~pathlib.Path, *others: Map, select: Op=Op.Last) -> Buffer | None

        Build a new map that is the symmetric difference between ``self`` and ``others``.
        The resulting map will contain all keys that appear an odd number of times, i.e.,
        if only one other map is given, it will contain all keys that are in either ``self`` or ``others``, but not in both.
        ``others`` must be instances of :class:`Map`.
        ``select`` specifies how conflicts are resolved if keys are present more than once.
        If ``path`` is ``":memory:"``, returns a :class:`Buffer` containing the map data instead of writing to path.
        ``path`` can be :class:`str` or :class:`~pathlib.Path`.

    .. method:: union(self, path: str | ~pathlib.Path, *others: Map, select: Op=Op.Last) -> Buffer | None

        Build a new map that is the union of ``self`` and ``others``.
        ``others`` must be instances of :class:`Map`.
        ``select`` specifies how conflicts are resolved if keys are present more than once.
        If ``path`` is ``":memory:"``, returns a :class:`Buffer` containing the map data instead of writing to path.
        ``path`` can be :class:`str` or :class:`~pathlib.Path`.



.. class:: Op

    Conflict resolution strategies for set operations on maps.

    .. attribute:: Avg

        Select average, i.e., ``sum(values) // len``.

    .. attribute:: First

        Select first value.

    .. attribute:: Last

        Select last value.

    .. attribute:: Max

        Select maximum.

    .. attribute:: Median

        Select median, i.e., with ``values = sorted(values)`` and ``mid = len // 2``,
        select ``values[mid]`` for odd length,
        and ``(values[mid-1] + values[mid]) // 2`` for even length.

    .. attribute:: Mid

        Select middle value, i.e., ``values[len // 2]``.

    .. attribute:: Min

        Select minimum.



.. class:: Set(data: SupportsBytes)

    An immutable set of bytes keys, based on finite-state-transducers.
    Typically uses a fraction of the memory as the builtin ``set`` and can be streamed from a file.

    ``data`` can be any object that supports the buffer protocol,
    e.g., :class:`Buffer`, :class:`bytes`, :class:`memoryview`, :class:`~mmap.mmap`, etc.
    Use :meth:`Map.build` to create suitable ``data``.

    .. important:: ``data`` needs to be contiguous.

    To the extent that it's feasible, ducer sets are intended to be direct replacements for the builtin :class:`set`.
    For ``s, o: Set``, and ``k: bytes``, the following works as intended::

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

    * ``add``
    * ``clear``
    * ``difference_update``, ``-=``
    * ``discard``
    * ``intersection_update``, ``&=``
    * ``pop``
    * ``remove``
    * ``symmetric_difference_update``, ``^=``
    * ``update``, ``|=``

    Further, the ``|``, ``&``, ``-``, ``^`` operators are also not implemented,
    since it is not possible to specify the storage path.
    Use :meth:`Set.union`, :meth:`Set.intersection`, :meth:`Set.difference`, and :meth:`Set.symmetric_difference` instead.

    .. classmethod:: build(cls, path: str | ~pathlib.Path, iterable: Iterable[SupportsBytes]) -> Buffer | None

        Build a set from an iterable of :class:`bytes` and write it to the given path.
        If ``path`` is ``":memory:"``, returns a :class:`Buffer` containing the set data.
        ``path`` can be :class:`str` or :class:`~pathlib.Path`.

    .. method:: copy(self) -> Set

        Since sets are immutable, returns self.

    .. method:: keys(self) -> Iterator[bytes]

        Iterate over all keys.

    .. method:: range(self, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[bytes]

        Iterate over all keys with optional range limits
        ``ge`` (greater than or equal),
        ``gt`` (greater than),
        ``le`` (less than or equal),
        and ``lt`` (less than).
        If no limits are given this is equivalent to ``iter(self)``.

    .. method:: starts_with(self, str: bytes, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[bytes]

        Iterate over all keys that start with :class:`str`.
        Optionally apply range limits
        ``ge`` (greater than or equal),
        ``gt`` (greater than),
        ``le`` (less than or equal),
        and ``lt`` (less than).

    .. method:: subsequence(self, str: bytes, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[bytes]

        Iterate over all keys that contain the subsequence :class:`str`.
        Keys don't need to contain the subsequence consecutively,
        e.g., ``b"bd"`` will match the key ``b"abcde"``.
        Optionally apply range limits
        ``ge`` (greater than or equal),
        ``gt`` (greater than),
        ``le`` (less than or equal),
        and ``lt`` (less than).

    .. method:: search(self, automaton: Automaton, ge: bytes | None=None, gt: bytes | None=None, le: bytes | None=None, lt: bytes | None=None) -> Iterator[bytes]

        Iterate over all keys that match the given ``Automaton``.
        Optionally apply range limits
        ``ge`` (greater than or equal),
        ``gt`` (greater than),
        ``le`` (less than or equal),
        and ``lt`` (less than).

    .. method:: difference(self, path: str | ~pathlib.Path, *others: Set) -> Buffer | None

        Build a new set that is the difference between ``self`` and all ``others``,
        meaning the resulting set will contain all keys that are in ``self``, but not in ``others``.
        ``others`` must be instances of :class:`Set`.
        If ``path`` is ``":memory:"``, returns a :class:`Buffer` containing the set data instead of writing to path.
        ``path`` can be :class:`str` or :class:`~pathlib.Path`.

    .. method:: intersection(self, path: str | ~pathlib.Path, *others: Set) -> Buffer | None

        Build a new set that is the intersection of ``self`` and ``others``.
        ``others`` must be instances of :class:`Set`.
        If ``path`` is ``":memory:"``, returns a :class:`Buffer` containing the set data instead of writing to path.
        ``path`` can be :class:`str` or :class:`~pathlib.Path`.

    .. method:: symmetric_difference(self, path: str | ~pathlib.Path, *others: Set) -> Buffer | None

        Build a new set that is the symmetric difference between ``self`` and ``others``,
        The resulting set will contain all keys that appear an odd number of times, i.e.,
        if only one other set is given, it will contain all keys that are in either ``self`` or ``others``, but not in both.
        ``others`` must be instances of :class:`Set`.
        If ``path`` is ``":memory:"``, returns a :class:`Buffer` containing the set data instead of writing to path.
        ``path`` can be :class:`str` or :class:`~pathlib.Path`.

    .. method:: union(self, path: str | ~pathlib.Path, *others: Set) -> Buffer | None

        Build a new set that is the union of ``self`` and ``others``.
        ``others`` must be instances of :class:`Set`.
        If ``path`` is ``":memory:"``, returns a :class:`Buffer` containing the set data instead of writing to path.
        ``path`` can be :class:`str` or :class:`~pathlib.Path`.
