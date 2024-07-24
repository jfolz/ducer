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
data = ducer.Map.build(items, ":memory:")
print(len(data))
```

Creating this map takes about 4 minutes on my humble laptop,
which translates to almost 4 Million items per second.
About 3 minutes is spent in Python just creating the item tuples.
This is a pathological scenario, the resulting output is just 464 bytes.
A real-world example with 1.1 Billion keys, where the msgpacked
key-value pairs occupy 21 GiB (without any kind of searchability),
results in a 4.6 GiB file.



## Limitations

Performance is rarely free,
so there are some limitations you should consider before proceeding:

* Keys **must** be `bytes`
* Keys **must** be inserted in lexicographical order
* Map values **must** be positive integers less than 2^64
* Once built, maps and sets **cannot** be altered



## Usage

### Building & opening

Above, we created a map with data stored in memory.
If you want to store your map in a file, you can also give a path instead:

```Python
ducer.Map.build(items, "path/to/my.map")
```

Building a map like this is a very memory-efficient.
You can also stream a map without loading all data into memory,
e.g., using the builtin mmap:

```Python
with open("path/to/my.map", "rb") as f:
    mm = mmap.mmap(f.fileno(), 0, access=mmap.ACCESS_READ)
    m = ducer.Map(mm)
```

### Access

Other than being read-only, maps and sets behave the same as the builtin
Python `dict` and `set`.
Please open an issue if you find something that doesn't work.
For map `m: Map` and key `k: bytes`, the following all work as intended:

```Python
k in m  # bool
m[k]  # value except KeyError
m.get(k)  # value or None
m.get(k, 0)  # value or 0
len(m)  # number of items
for k in m:  # iterate over keys
    pass
for v in m.values():  # iterate over values
    pass
for k, v in m.items():  # iterate over items
    pass
```

### TODO advanced search & operations
