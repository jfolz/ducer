# Ducer documentation

This package provides fast and compact read-only
[maps](https://ducer.readthedocs.io/stable/api_reference.html#Map)
and
[sets](https://ducer.readthedocs.io/stable/api_reference.html#Set)
that scale up to Billions of keys, while being light on resources
with a streamable file format.
Complex search patterns that would be infeasible with Python's builtin
[`dict`](https://docs.python.org/3/library/stdtypes.html#dict)
and [`set`](https://docs.python.org/3/library/stdtypes.html#set)
are not just possible, but very efficient.
All of these amazing things are achieved with finite-state-transducers
provided by the excellent Rust crate
[fst](https://github.com/BurntSushi/fst) by Andrew Gallant.



## Performance

Ducer maps and sets can be built and queried at millions of keys per second.
Consider the following example:

```Python
import ducer
n = 1_000_000_000
items = ((b"%09d" % i, n-i) for i in range(n))
data = ducer.Map.build(":memory:", items)
m = Map(data)
assert m[b"900000000"] == 100_000_000
```

In our example, most of the time is spent in Python creating item tuples.
Regardless, building happens at almost 4 Million items per second
on my humble laptop.
Retrieving individual keys is similarly speedy, and simply iterating
over all items is twice as fast at 8 Million items per second.

This toy example is almost a best case for FSTs, so the resulting
output is just 464 bytes.
A real-world example with 1.1 Billion keys, where the key-value pairs
occupy 21 GiB without any kind of searchability (stored in already quite
compact [msgpack](https://github.com/msgpack/msgpack-python) format),
results in a 4.6 GiB file.
Building and retrieval are naturally a bit slower,
but still within the 2-3 Million items per second range.



## Limitations

Performance is rarely free,
so there are some limitations you should consider before proceeding:

* Keys **must** be **[`bytes`](https://docs.python.org/3/library/stdtypes.html#bytes)**
* Keys **must** be inserted in lexicographical order
* Map values **must** be non-negative integers less than 2^64
* Once built, maps and sets **cannot** be altered



## Installation

Most users should be able to simply do:

```
pip install ducer
```

To build from source you will need a recent Rust toolchain.
Use your preferred method of installation, or follow the official
[instructions](https://www.rust-lang.org/tools/install) to install Rust.
Then run the following at the toplevel of the repository:

```
pip install .
```



## Building

Above, we already showed that
[`Map.build`](https://ducer.readthedocs.io/stable/api_reference.html#Map.build)
can build maps in memory by passing
`":memory:"` as path:

```Python
data = ducer.Map.build(":memory:", items)
```

If you pass any other path
(either [`str`](https://docs.python.org/3/library/stdtypes.html#str)
or [`Path`](https://docs.python.org/3/library/pathlib.html#pathlib.Path);
the parent directory must exist),
your map will be written directly to that file:

```Python
ducer.Map.build("path/to/my.map", items)
```

Building a map like this uses virtually no extra memory.



## Opening

One key advantage of ducer maps is streamability.
Unlike the builtin [`dict`](https://docs.python.org/3/library/stdtypes.html#dict),
a [`Map`](https://ducer.readthedocs.io/stable/api_reference.html#Map)
does not have to reside entirely in memory.
You can, e.g., use the builtin
[`mmap`](https://docs.python.org/3/library/mmap.html#mmap.mmap) to stream map data:

```Python
with open("path/to/my.map", "rb") as f:
    mm = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
m = ducer.Map(mm)
```

Note that you can safely close the file once the
[`mmap`](https://docs.python.org/3/library/mmap.html#mmap.mmap) is created.
See [mmap(2)](https://www.man7.org/linux/man-pages/man2/mmap.2.html) for details.

Thanks to high compression ratios, pre-loading maps entirely into memory can be feasible.
In our experience, at least for local SSD storage, performance is virtually
identical, expect pre-loading data takes extra time.
Pre-loading can still make sense though, e.g., with networked storage.

```Python
with open("path/to/my.map", "rb") as f:
    data = f.read()
m = ducer.Map(data)
```



## Access

To the extent that it's feasible,
[`Map`](https://ducer.readthedocs.io/stable/api_reference.html#Map)
and [`Set`](https://ducer.readthedocs.io/stable/api_reference.html#Set)
are intended to be direct replacements for the builtin Python
[`dict`](https://docs.python.org/3/library/stdtypes.html#dict)
and [`set`](https://docs.python.org/3/library/stdtypes.html#set).
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
s < o  # proper subset
s.issuperset(o)
s >= o  # superset
s > o  # proper superset
```

**Note:** Comparison operations are currently only implemented
for other [`Set`](https://ducer.readthedocs.io/stable/api_reference.html#Set)
objects, not the builtin
[`set`](https://docs.python.org/3/library/stdtypes.html#set).
This may change in a future version if there is demand for it.



## Differences to builtins

### Not implemented

Since [`Map`](https://ducer.readthedocs.io/stable/api_reference.html#Map)
is immutable, the following are **not implemented**:
- `clear`
- `fromkeys`
- `pop`
- `popitem`
- `setdefault`
- `update`, `|=`

Since [`Set`](https://ducer.readthedocs.io/stable/api_reference.html#Set)
is immutable, the following are **not implemented**:
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
Use the respective `union`, `intersection`, `difference`,
and `symmetric_difference` methods instead.


### Incompatible syntax

`difference`, `intersection`, `symmetric_difference`, and `union`
have slightly different syntax to accomodate the necessary path.
For `s: Set` and `others: Iterable[Set]`:

```Python
s.difference("path/to/result.set", *others)
s.intersection("path/to/result.set", *others)
s.symmetric_difference("path/to/result.set", *others)
s.union("path/to/result.set", *others)
```

Like the standard library, difference will create the set of
all elements of `s` that are not present in `others`.


### Set operations on maps

Unlike the builtin [`dict`](https://docs.python.org/3/library/stdtypes.html#dict),
the ducer [`Map`](https://ducer.readthedocs.io/stable/api_reference.html#Map)
offers set operations.
The syntax is the same as for sets:

```Python
m.difference("path/to/result.map", *others)
m.intersection("path/to/result.map", *others)
m.symmetric_difference("path/to/result.map", *others)
m.union("path/to/result.map", *others)
```

To resolve conflicts between keys present in multiple maps,
a list of possible values is assembled.
If the key is present in `self`, then it will be the first value.
Values from `others` are added in given order.
By default the last values in the list is used to mimic the behavior
of [`dict.update`](https://docs.python.org/3/library/stdtypes.html#dict.update).
Currently, you can choose between these pre-defined operations:

- [`ducer.Op.First`](https://ducer.readthedocs.io/stable/api_reference.html#Op.First) -- first element
- [`ducer.Op.Mid`](https://ducer.readthedocs.io/stable/api_reference.html#Op.Mid) -- middle element, left if even number of values
- [`ducer.Op.Last`](https://ducer.readthedocs.io/stable/api_reference.html#Op.Last) -- the default
- [`ducer.Op.Min`](https://ducer.readthedocs.io/stable/api_reference.html#Op.Min) -- minimum value
- [`ducer.Op.Avg`](https://ducer.readthedocs.io/stable/api_reference.html#Op.Avg) -- average value cast to `int`
- [`ducer.Op.Median`](https://ducer.readthedocs.io/stable/api_reference.html#Op.Median) -- median value cast to `int`
- [`ducer.Op.Max`](https://ducer.readthedocs.io/stable/api_reference.html#Op.Max) -- maximum value

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



## Advanced search patterns

The underlying FSTs allow for some advanced search patterns that would
otherwise be costly to implement on top of
[`dict`](https://docs.python.org/3/library/stdtypes.html#dict)
and [`set`](https://docs.python.org/3/library/stdtypes.html#set).
Most basic, you can iterate over a range of keys, where
`ge` = greater or equals,
`gt` = greater than,
`le` = less than or equals,
and `lt` = less than:

```Python
m.range(ge=b"key17", lt=b"key42")
m.range(gt=b"key17", le=b"key42")
```

For maps this yields key-value tuples, meaning
[`m.range()`](https://ducer.readthedocs.io/stable/api_reference.html#Map.range)
without limits is equivalent to
[`m.items()`](https://ducer.readthedocs.io/stable/api_reference.html#Map.items).

You can also iterate over all keys that
[start with](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.starts_with)
a certain prefix,
with optional limits same as `range`:

```Python
m.starts_with(b"key", ge=b"key17", lt=b"key42")
m.starts_with(b"key", gt=b"key17", le=b"key42")
```

You can also search for
[subsequences](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.subsequence)
, e.g., all keys matching `*k*7*`,
again with optional limits:

```Python
m.subsequence(b"k7", ge=b"key17", lt=b"key42")
m.subsequence(b"k7", gt=b"key17", le=b"key42")
```

Finally, you can create an `Automaton` to create your own search patterns.
The following automata are available:

- [`always`](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.always)
- [`never`](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.never)
- [`str`](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.str)
- [`subsequence`](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.subsequence)
- [`complement`](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.complement)
- [`starts_with`](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.starts_with)
- [`intersection`](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.intersection)
- [`union`](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.union)

For example, to recreate
[`Map.starts_with`](https://ducer.readthedocs.io/stable/api_reference.html#Map.starts_with),
you can use the following
automata with the
[`Map.search`](https://ducer.readthedocs.io/stable/api_reference.html#Map.search)
method:

```Python
a = ducer.Automaton.str(b"key").starts_with()
m.search(a)
```

Add
[`complement`](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.complement)
to search for keys that do not start with the string:

```Python
a = ducer.Automaton.str(b"key").starts_with().complement()
m.search(a)
```

Finally, you can combine multiple automata, e.g. with
[`union`](https://ducer.readthedocs.io/stable/api_reference.html#Automaton.union):

```Python
a1 = ducer.Automaton.str(b"key").starts_with()
a2 = ducer.Automaton.str(b"other").starts_with()
a = a1.union(a2)
m.search(a)
```



## Acknowledgements

Ducer is supported by the [SustainML](https://sustainml.eu/) project.
