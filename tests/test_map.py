from __future__ import annotations

import contextlib
import mmap
from pathlib import Path

import pytest

from ducer import Automaton, Map, Op


K1 = b"key1"
V1 = 123
I1 = K1, V1

K2 = b"key2"
V2 = 456
I2 = K2, V2

K3 = b"key3"
V3 = 789
I3 = K3, V3

KO = b"other"
VO = 123456789
IO = KO, VO

DICT12 = dict((I1, I2))
DICT123 = dict((I1, I2, I3))
DICT23 = dict((I2, I3))
DICT12O = dict((I1, I2, IO))


def build_map(source=DICT12, path: str | Path = ":memory:"):
    return Map.build(path, source.items())


def create_map(source=DICT12):
    return Map(build_map(source=source))


def validate_map(s, source=DICT12):
    for k in source:
        assert k in s
    for k in s:
        assert k in source


def validate_map_file(path, source=DICT12):
    with open(path, "rb") as f:
        data = f.read()
    s = Map(data)
    validate_map(s, source=source)


def test_map_build_memory():
    validate_map(create_map())


def test_map_build_str(tmp_path):
    path = str(tmp_path / "test.map")
    build_map(path=path)
    validate_map_file(path)


def test_map_build_path(tmp_path):
    path = Path(tmp_path / "test.map")
    build_map(path=path)
    validate_map_file(path)


def test_map_build_not_bytes():
    with pytest.raises(TypeError):
        Map.build(":memory:", [("key", 0)])


def test_map_build_not_int():
    with pytest.raises(TypeError):
        Map.build(":memory:", [(b"key", 0.5)])


def test_map_build_negative():
    with pytest.raises(OverflowError):
        Map.build(":memory:", [(b"key", -1)])


def test_map_build_too_large():
    with pytest.raises(OverflowError):
        Map.build(":memory:", [(b"key", 2**64)])


def test_map_build_buffer_file(tmp_path):
    path = tmp_path / "test.map"
    build_map(path=path)
    with open(path, "rb") as f:
        data = f.read()
    validate_map(Map(data))


@contextlib.contextmanager
def init_mmap(tmp_path):
    path = tmp_path / "test.map"
    build_map(path=path)
    with open(path, "rb") as fp:
        mm = mmap.mmap(fp.fileno(), 0, access=mmap.ACCESS_READ)
        yield Map(mm)


def test_map_init_mmap(tmp_path):
    with init_mmap(tmp_path) as m:
        pass


def test_map_len_memory():
    m = create_map()
    assert len(m) == 2


def test_map_len_mmap(tmp_path):
    with init_mmap(tmp_path) as m:
        assert len(m) == 2


def test_map_contains():
    m = create_map()
    for k in DICT12:
        assert k in m
    assert K3 not in m


def test_map_getitem_contained():
    m = create_map()
    for k, v in DICT12.items():
        assert m[k] == v


def test_map_getitem_missing():
    m = create_map()
    with pytest.raises(KeyError):
        m[K3]


def test_map_get_contained():
    m = create_map()
    for k, v in DICT12.items():
        assert m.get(k) == v


def test_map_get_contained_default():
    m = create_map()
    for k, v in DICT12.items():
        assert m.get(k, default=17) == v


def test_map_get_missing():
    m = create_map()
    assert m.get(K3) is None


def test_map_get_missing_default():
    m = create_map()
    assert m.get(K3, 17) == 17


def test_map_iter():
    m = create_map()
    for k1, k2 in zip(m, sorted(DICT12)):
        assert k1 == k2


def test_map_iter_mmap(tmp_path):
    with init_mmap(tmp_path) as m:
        for k1, k2 in zip(m, sorted(DICT12)):
            assert k1 == k2


def test_map_keys():
    m = create_map()
    for k1, k2 in zip(m.keys(), sorted(DICT12)):
        assert k1 == k2


def test_map_values():
    m = create_map()
    for v1, v2 in zip(m.values(), (v for _, v in sorted(DICT12.items()))):
        assert v1 == v2


def test_map_items():
    m = create_map()
    for i1, i2 in zip(m.items(), sorted(DICT12.items())):
        assert i1 == i2


def test_map_range():
    m = create_map()
    for i1, i2 in zip(m.range(), sorted(DICT12.items())):
        assert i1 == i2


def test_map_range_lt():
    m = create_map()
    items = list(m.range(lt=K2))
    assert I1 in items
    assert I2 not in items


def test_map_range_le():
    m = create_map()
    items = list(m.range(le=K1))
    assert I1 in items
    assert I2 not in items


def test_map_range_gt():
    m = create_map()
    items = list(m.range(gt=K1))
    assert I1 not in items
    assert I2 in items


def test_map_range_ge():
    m = create_map()
    items = list(m.range(ge=K2))
    assert I1 not in items
    assert I2 in items


def test_map_range_lt_gt():
    m = create_map()
    items = list(m.range(lt=K2, gt=K1))
    assert not items


def test_map_range_le_gt():
    m = create_map()
    items = list(m.range(le=K2, gt=K1))
    assert I1 not in items
    assert I2 in items


def test_map_range_lt_ge():
    m = create_map()
    items = list(m.range(lt=K2, ge=K1))
    assert I1 in items
    assert I2 not in items


def test_map_search_always():
    m = create_map()
    a = Automaton.always()
    items = list(m.search(a))
    for i in DICT12.items():
        assert i in items


def test_map_search_always_complement():
    m = create_map()
    a = Automaton.always().complement()
    items = list(m.search(a))
    assert not items


def test_map_search_never():
    m = create_map()
    a = Automaton.never()
    items = list(m.search(a))
    assert not items


def test_map_search_never_complement():
    m = create_map()
    a = Automaton.never().complement()
    items = list(m.search(a))
    for i in DICT12.items():
        assert i in items


def test_map_search_str():
    m = create_map()
    a = Automaton.str(K1)
    items = list(m.search(a))
    assert I1 in items
    assert I2 not in items


def test_map_search_str_complement():
    m = create_map()
    a = Automaton.str(K1).complement()
    items = list(m.search(a))
    assert I1 not in items
    assert I2 in items


def test_map_search_subsequence():
    m = create_map()
    a = Automaton.subsequence(b"k1")
    items = list(m.search(a))
    assert I1 in items
    assert I2 not in items


def test_map_search_subsequence_complement():
    m = create_map()
    a = Automaton.subsequence(b"k1").complement()
    items = list(m.search(a))
    assert I1 not in items
    assert I2 in items


def test_map_search_starts_with():
    m = create_map(source=DICT12O)
    a = Automaton.str(b"key").starts_with()
    items = list(m.search(a))
    assert I1 in items
    assert I2 in items
    assert IO not in items


def test_map_search_starts_with_complement():
    m = create_map(source=DICT12O)
    a = Automaton.str(b"key").starts_with().complement()
    items = list(m.search(a))
    assert I1 not in items
    assert I2 not in items
    assert IO in items


def test_map_search_union():
    m = create_map(source=DICT12O)
    a1 = Automaton.str(K1)
    a2 = Automaton.str(b"oth").starts_with()
    a = a1.union(a2)
    items = list(m.search(a))
    assert I1 in items
    assert I2 not in items
    assert IO in items


def test_map_search_intersection():
    m = create_map(source=DICT123)
    a1 = Automaton.str(K1).complement()
    a2 = Automaton.str(K3).complement()
    a = a1.intersection(a2)
    items = list(m.search(a))
    assert I1 not in items
    assert I2 in items
    assert I3 not in items


def test_map_difference():
    m1 = create_map(source=DICT123)
    m2 = create_map(source=DICT23)
    m = Map(m1.difference(":memory:", m2))
    items = list(m.items())
    assert I1 in items
    assert I2 not in items
    assert I3 not in items


def test_map_intersection():
    m1 = create_map(source=DICT12)
    m2 = create_map(source=DICT23)
    m = Map(m1.intersection(":memory:", m2))
    items = list(m.items())
    assert I1 not in items
    assert I2 in items
    assert I3 not in items


def test_map_symmetric_difference():
    m1 = create_map(source=DICT12)
    m2 = create_map(source=DICT23)
    m = Map(m1.symmetric_difference(":memory:", m2))
    items = list(m.items())
    assert I1 in items
    assert I2 not in items
    assert I3 in items


def test_map_union():
    m1 = create_map(source=DICT12)
    m2 = create_map(source=DICT23)
    m = Map(m1.union(":memory:", m2))
    items = list(m.items())
    assert I1 in items
    assert I2 in items
    assert I3 in items


def op_test_maps():
    return (
        Map(Map.build(":memory:", {K1: V1}.items())),
        Map(Map.build(":memory:", {K1: V2}.items())),
        Map(Map.build(":memory:", {K1: V3}.items())),
    )


def test_map_union_multiple_first():
    m1, *ms = op_test_maps()
    m = Map(m1.union(":memory:", *ms, select=Op.First))
    assert m[K1] == V1


def test_map_union_multiple_mid():
    m1, *ms = op_test_maps()
    m = Map(m1.union(":memory:", *ms, select=Op.Mid))
    assert m[K1] == V2


def test_map_union_multiple_last():
    m1, *ms = op_test_maps()
    m = Map(m1.union(":memory:", *ms, select=Op.Last))
    assert m[K1] == V3


def test_map_union_multiple_min():
    m1, *ms = op_test_maps()
    m = Map(m1.union(":memory:", *ms, select=Op.Min))
    assert m[K1] == V1


def test_map_union_multiple_avg():
    m1, *ms = op_test_maps()
    m = Map(m1.union(":memory:", *ms, select=Op.Avg))
    assert m[K1] == (V1 + V2 + V3) // 3


def test_map_union_multiple_max():
    m1, *ms = op_test_maps()
    m = Map(m1.union(":memory:", *ms, select=Op.Max))
    assert m[K1] == V3


def test_map_union_multiple_median_odd():
    m1, *ms = op_test_maps()
    m = Map(m1.union(":memory:", *ms, select=Op.Median))
    assert m[K1] == V2


def test_map_union_multiple_median_even():
    m1, m2, _ = op_test_maps()
    m = Map(m1.union(":memory:", m2, select=Op.Median))
    assert m[K1] == (V1 + V2) // 2


def test_map_eq_true():
    s1 = create_map(source=DICT12)
    s2 = create_map(source=DICT12)
    assert s1 == s2


def test_map_eq_false():
    s1 = create_map(source=DICT12)
    s2 = create_map(source=DICT123)
    assert s1 != s2
    assert s2 != s1
    s1 = create_map(source=DICT12)
    s2 = create_map(source=DICT23)
    assert s1 != s2
    assert s2 != s1


def test_map_eq_false_other():
    s = create_map()
    assert s != 7
