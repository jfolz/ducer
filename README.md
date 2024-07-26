# Ducer

This package provides Python bindings for the excellent Rust crate
[fst](https://github.com/BurntSushi/fst) by Andrew Gallant.
`Map` and `Set` classes allow building compact representations from sorted
Python iterables.



## Performance

Ducer maps and sets can be built and queried at millions of keys per second.
Consider the following example:

```Python
n = 1_000_000_000
items = ((b"%09d" % i, n-i) for i in range(n))
data = ducer.Map.build(":memory:", items)
```

Creating this map takes about 4 minutes on my humble laptop,
which translates to almost 4 Million items per second.
About 3 minutes is spent in Python just creating the item tuples.
This scenario is almost a best case for FSTs, so the resulting
output is just 464 bytes.
A real-world example with 1.1 Billion keys, where the
[msgpacked](https://github.com/msgpack/msgpack-python)
key-value pairs occupy 21 GiB (without any kind of searchability),
results in a 4.6 GiB file.



## Limitations

Performance is rarely free,
so there are some limitations you should consider before proceeding:

* Keys **must** be `bytes`
* Keys **must** be inserted in lexicographical order
* Map values **must** be non-negative integers less than 2^64
* Once built, maps and sets **cannot** be altered



## Usage

### Building

Above, we already showed that you can build maps in memory by passing
`":memory:"` as path:

```Python
data = ducer.Map.build(":memory:", items)
```

If you pass any other path
(either `str` or `pathlib.Path`; the parent directory must exist),
your map will be written directly to that file:

```Python
ducer.Map.build("path/to/my.map", items)
```

Building a map like this uses virtually no extra memory.


### Opening

One key advantage of ducer maps is streamability.
Unlike the builtin `dict`, a `Map` does not have to reside entirely in memory.
You can, e.g., use the builtin mmap to stream map data:

```Python
with open("path/to/my.map", "rb") as f:
    mm = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
    m = ducer.Map(mm)
```

High compression ratios can, however, allow storing maps entirely in memory.
In our experience, at least for fast SSD storage, performance is virtually
identical, expect reading the file into memory takes extra time.

```Python
with open("path/to/my.map", "rb") as f:
    data = f.read()
m = ducer.Map(data)
```


### Access

To the extent that it's feasible, `Map` and `Set` are intended to be
direct replacements for the builtin Python `dict` and `set`.
For `m, o: Map` and `k: bytes`, the following works as intended:

```Python
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
```

For `s, o: Set`, and `k: bytes`, the following works as intended:

```Python
k in s
s == o
len(s)
for k in s:
    pass
s.isdisjoint(o)
s.issubset(o)
s <= o  # subset
s < o  # true subset
s.issuperset(o)
s >= o  # superset
s > o  # true superset
```

**Note:** Comparison operations are currently only implemented
for other `Set` objects, not the builtin `set`.
This may change in a future version if there is demand for it.


### Differences to builtins

#### Not implemented

Since `Map` is immutable, the following are **are not implemented**:
- `clear`
- `fromkeys`
- `pop`
- `popitem`
- `setdefault`
- `update`

Since `Set` is immutable, the following are **are not implemented**:
- `add`
- `clear`
- `difference_update`, `-=`
- `discard`
- `intersection_update`, `&=`
- `pop`
- `remove`
- `symmetric_difference_update`, `^=`
- `update`, `|=`

Further, the `|`, `&`, `-`, `^` operators are also not implemented,
since it is not possible to specify the storage path.


#### Incompatible syntax

`difference`, `intersection`, `symmetric_difference`, and `union`
have slightly different syntax to accomodate the necessary path.
For `s: Set` and `others: Iterable[Set]`:

```Python
s.difference("path/to/my.set", *others)
s.intersection("path/to/my.set", *others)
s.symmetric_difference("path/to/my.set", *others)
s.union("path/to/my.set", *others)
```

Like the standard library, difference will create the set of
all elements of `s` that are not present in `others`.


#### Set operations on maps

Unlike the builtin `dict`, the ducer `Map` offers set operations.
The syntax is the same as for sets:

```Python
m.difference("path/to/my.map", *others)
m.intersection("path/to/my.map", *others)
m.symmetric_difference("path/to/my.map", *others)
m.union("path/to/my.map", *others)
```

To resolve conflicts between keys present in multiple maps,
a list of possible values is assembled.
If the key is present in `self`, then it will be the first value.
Values from `others` are added in given order.
By default the last values in the list is used to mimic the behavior
of `dict.update`.
Currently, you can choose between these pre-defined operations:

- `ducer.Op.First`
- `ducer.Op.Mid` -- middle element, left if even number of values
- `ducer.Op.Last` -- the default
- `ducer.Op.Min`
- `ducer.Op.Avg` -- average value cast to `int`
- `ducer.Op.Median` -- median value cast to `int`
- `ducer.Op.Max`

Some examples:

```Python
m1 = ducer.Map(ducer.Map.build(":memory:", [(b"k1", 1), (b"k2", 1)]))
m2 = ducer.Map(ducer.Map.build(":memory:", [(b"k2", 2), (b"k3", 2)]))
m3 = ducer.Map(ducer.Map.build(":memory:", [(b"k3", 3)]))
mu = ducer.Map(m1.union(":memory:", m2, m3))
print(dict(mu.items()))
# {b'k1': 1, b'k2': 2, b'k3': 3}
mu = ducer.Map(m1.union(":memory:", m2, m3, select=ducer.Op.First))
print(dict(mu.items()))
# {b'k1': 1, b'k2': 1, b'k3': 2}
```


### Advanced search patterns

The underlying FSTs allow for some advanced search patterns that would
otherwise be costly to implement on top of `dict` and `set`.
Most basic, you can iterate over a range of keys
(where ge = greater or equals, gt = greater than,
le = less than or equals, lt = less than):

```Python
m.range(ge=b"key17", lt=b"key42")
m.range(gt=b"key17", le=b"key42")
```

For maps `range` yields key-value tuples, meaning `m.range()` without
limits is equivalent to `m.items()`.

You can also iterate over all keys that start with a certain prefix,
with optional limits same as `range`:

```Python
m.starts_with(b"key", ge=b"key17", lt=b"key42")
m.starts_with(b"key", gt=b"key17", le=b"key42")
```

You can also search for subsequences, e.g., all keys matching `*k*7*`,
again with optional limits:

```Python
m.subsequence(b"k7", ge=b"key17", lt=b"key42")
m.subsequence(b"k7", gt=b"key17", le=b"key42")
```

Finally, you can create an `Automaton` to create your own search patterns.
The following automata are available:

- always
- never
- str
- subsequence
- complement
- starts_with
- intersection
- union

For example, to recreate `Map.starts_with`, you can use the following
automata with the `Map.search` method:

```Python
a = ducer.Automaton.str(b"key").starts_with()
m.search(a)
```

Add `complement` to search for keys that do not start with the string:

```Python
a = ducer.Automaton.str(b"key").starts_with().complement()
m.search(a)
```

Finally, you can combine multiple automata, e.g. with `union`:

```Python
a1 = ducer.Automaton.str(b"key").starts_with()
a2 = ducer.Automaton.str(b"other").starts_with()
a = a1.union(a2)
m.search(a)
```
