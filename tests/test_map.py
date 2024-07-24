import mmap
import pytest
import contextlib

import ducer


DICT = {b"key1": 123, b"key2": 456}


def build_map(source=DICT, path=":memory:"):
    return ducer.Map.build(source.items(), path)


def create_map(source=DICT):
    return ducer.Map(build_map(source=source))


def test_map_build_memory():
    build_map()


def test_map_build_file(tmp_path):
    build_map(path=tmp_path / "test.map")


def test_map_init_memory():
    create_map()


def test_map_init_read(tmp_path):
    path = tmp_path / "test.map"
    build_map(path=path)
    with open(path, "rb") as f:
        data = f.read()
    ducer.Map(data)


@contextlib.contextmanager
def init_mmap(tmp_path):
    path = tmp_path / "test.map"
    build_map(path=path)
    with open(path, "rb") as fp:
        mm = mmap.mmap(fp.fileno(), 0, prot=mmap.PROT_READ)
        mm.madvise(mmap.MADV_RANDOM)
        yield ducer.Map(mm)


def test_map_init_mmap(tmp_path):
    with init_mmap(tmp_path) as m:
        pass


def test_map_len_memory():
    m = create_map()
    assert len(m) == 2


def test_map_len_mmap(tmp_path):
    with init_mmap(tmp_path) as m:
        assert len(m) == 2


def test_map_getitem_contained():
    m = create_map()
    for k, v in DICT.items():
        assert m[k] == v


def test_map_getitem_missing():
    m = create_map()
    with pytest.raises(KeyError):
        m[b"key3"]


def test_map_get_contained():
    m = create_map()
    for k, v in DICT.items():
        assert m.get(k) == v


def test_map_get_contained_default():
    m = create_map()
    for k, v in DICT.items():
        assert m.get(k, default=17) == v


def test_map_get_missing():
    m = create_map()
    assert m.get(b"key3") is None


def test_map_get_missing_default():
    m = create_map()
    assert m.get(b"key3", 17) == 17


def test_map_iter():
    m = create_map()
    for k1, k2 in zip(m, sorted(DICT)):
        assert k1 == k2


def test_map_iter_mmap(tmp_path):
    with init_mmap(tmp_path) as m:
        for k1, k2 in zip(m, sorted(DICT)):
            assert k1 == k2


def test_map_keys():
    m = create_map()
    for k1, k2 in zip(m.keys(), sorted(DICT)):
        assert k1 == k2


def test_map_values():
    m = create_map()
    for v1, v2 in zip(m.values(), (v for _, v in sorted(DICT.items()))):
        assert v1 == v2


def test_map_items():
    m = create_map()
    for i1, i2 in zip(m.items(), sorted(DICT.items())):
        assert i1 == i2


def test_map_range():
    m = create_map()
    for i1, i2 in zip(m.range(), sorted(DICT.items())):
        assert i1 == i2


def test_map_range_lt():
    m = create_map()
    items = list(m.range(lt=b"key2"))
    assert (b"key1", 123) in items
    assert (b"key2", 456) not in items


def test_map_range_le():
    m = create_map()
    items = list(m.range(le=b"key1"))
    assert (b"key1", 123) in items
    assert (b"key2", 456) not in items


def test_map_range_gt():
    m = create_map()
    items = list(m.range(gt=b"key1"))
    assert (b"key1", 123) not in items
    assert (b"key2", 456) in items


def test_map_range_ge():
    m = create_map()
    items = list(m.range(ge=b"key2"))
    assert (b"key1", 123) not in items
    assert (b"key2", 456) in items


def test_map_range_lt_gt():
    m = create_map()
    items = list(m.range(lt=b"key2", gt=b"key1"))
    assert not items


def test_map_range_le_gt():
    m = create_map()
    items = list(m.range(le=b"key2", gt=b"key1"))
    assert (b"key1", 123) not in items
    assert (b"key2", 456) in items


def test_map_range_lt_ge():
    m = create_map()
    items = list(m.range(lt=b"key2", ge=b"key1"))
    assert (b"key1", 123) in items
    assert (b"key2", 456) not in items


def test_map_search_always():
    m = create_map()
    a = ducer.Automaton.always()
    items = list(m.search(a))
    for i in DICT.items():
        assert i in items


def test_map_search_always_complement():
    m = create_map()
    a = ducer.Automaton.always().complement()
    items = list(m.search(a))
    assert not items


def test_map_search_never():
    m = create_map()
    a = ducer.Automaton.never()
    items = list(m.search(a))
    assert not items


def test_map_search_never_complement():
    m = create_map()
    a = ducer.Automaton.never().complement()
    items = list(m.search(a))
    for i in DICT.items():
        assert i in items


def test_map_search_str():
    m = create_map()
    a = ducer.Automaton.str("key1")
    items = list(m.search(a))
    assert (b"key1", 123) in items
    assert (b"key2", 456) not in items


def test_map_search_str_complement():
    m = create_map()
    a = ducer.Automaton.str("key1").complement()
    items = list(m.search(a))
    assert (b"key1", 123) not in items
    assert (b"key2", 456) in items


def test_map_search_subsequence():
    m = create_map()
    a = ducer.Automaton.subsequence("k1")
    items = list(m.search(a))
    assert (b"key1", 123) in items
    assert (b"key2", 456) not in items


def test_map_search_subsequence_complement():
    m = create_map()
    a = ducer.Automaton.subsequence("k1").complement()
    items = list(m.search(a))
    assert (b"key1", 123) not in items
    assert (b"key2", 456) in items


def test_map_search_starts_with():
    d = dict(DICT)
    d.update({b"other": 789})
    m = create_map(source=d)
    a = ducer.Automaton.str("key").starts_with()
    items = list(m.search(a))
    assert (b"key1", 123) in items
    assert (b"key2", 456) in items
    assert (b"other", 789) not in items


def test_map_search_starts_with_complement():
    d = dict(DICT)
    d.update({b"other": 789})
    m = create_map(source=d)
    a = ducer.Automaton.str("key").starts_with().complement()
    items = list(m.search(a))
    assert (b"key1", 123) not in items
    assert (b"key2", 456) not in items
    assert (b"other", 789) in items


def test_map_search_union():
    d = dict(DICT)
    d.update({b"other": 789})
    m = create_map(source=d)
    a1 = ducer.Automaton.str("key1")
    a2 = ducer.Automaton.str("oth").starts_with()
    a = a1.union(a2)
    items = list(m.search(a))
    assert (b"key1", 123) in items
    assert (b"key2", 456) not in items
    assert (b"other", 789) in items


def test_map_search_intersection():
    d = dict(DICT)
    d.update({b"key3": 789})
    m = create_map(source=d)
    a1 = ducer.Automaton.str("key1").complement()
    a2 = ducer.Automaton.str("key3").complement()
    a = a1.intersection(a2)
    items = list(m.search(a))
    assert (b"key1", 123) not in items
    assert (b"key2", 456) in items
    assert (b"key3", 789) not in items
