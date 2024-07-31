from __future__ import annotations

import contextlib
import mmap
from pathlib import Path

import pytest

from ducer import Automaton, Op, Set


K1 = b"key1"
K2 = b"key2"
K3 = b"key3"
SO = "other"
KO = SO.encode('utf-8')

SET1 = K1,
SET12 = K1, K2
SET123 = K1, K2, K3
SET2 = K2,
SET23 = K2, K3
SET3 = K3,
SET12O = K1, K2, KO


def build_set(source=SET12, path: str | Path = ":memory:"):
    return Set.build(path, source)


def create_set(source=SET12):
    return Set(build_set(source=source))


def validate_set(s, source=SET12):
    for k in source:
        assert k in s
    for k in s:
        assert k in source


def validate_set_file(path, source=SET12):
    with open(path, "rb") as f:
        data = f.read()
    s = Set(data)
    validate_set(s, source=source)


def test_set_build_memory():
    validate_set(Set(build_set()))


def test_set_build_str(tmp_path):
    path = str(tmp_path / "test.set")
    build_set(path=path)
    validate_set_file(path)


def test_set_build_path(tmp_path):
    path = Path(tmp_path / "test.set")
    build_set(path=path)
    validate_set_file(path)


def test_set_build_not_bytes():
    with pytest.raises(TypeError):
        Set.build(":memory:", ["key"])


def test_set_build_buffer_file(tmp_path):
    path = tmp_path / "test.set"
    buf = build_set()
    with open(path, 'wb') as f:
        f.write(buf)
    with open(path, 'rb') as f:
        data = f.read()
    validate_set(Set(data))


@contextlib.contextmanager
def init_mmap(tmp_path):
    path = tmp_path / "test.set"
    build_set(path=path)
    with open(path, "rb") as fp:
        mm = mmap.mmap(fp.fileno(), 0, access=mmap.ACCESS_READ)
        yield Set(mm)


def test_set_init_mmap(tmp_path):
    with init_mmap(tmp_path) as m:
        pass


def test_set_len_memory():
    m = create_set()
    assert len(m) == 2


def test_set_len_mmap(tmp_path):
    with init_mmap(tmp_path) as m:
        assert len(m) == 2


def test_set_contains():
    m = create_set()
    for k in SET12:
        assert k in m
    assert K3 not in m


def test_set_iter():
    m = create_set()
    for k1, k2 in zip(m, sorted(SET12)):
        assert k1 == k2


def test_set_iter_mmap(tmp_path):
    with init_mmap(tmp_path) as m:
        for k1, k2 in zip(m, sorted(SET12)):
            assert k1 == k2


def test_set_isdisjoint_true():
    s1 = create_set(source=SET1)
    s2 = create_set(source=SET23)
    assert s1.isdisjoint(s2)


def test_set_isdisjoint_false():
    s1 = create_set(source=SET12)
    s2 = create_set(source=SET23)
    assert not s1.isdisjoint(s2)


def test_set_issubset_true():
    s1 = create_set(source=SET1)
    s2 = create_set(source=SET12)
    assert s1.issubset(s2)


def test_set_issubset_false():
    s1 = create_set(source=SET12)
    s2 = create_set(source=SET23)
    assert not s1.issubset(s2)


def test_set_issuperset_true():
    s1 = create_set(source=SET123)
    s2 = create_set(source=SET23)
    assert s1.issuperset(s2)


def test_set_issuperset_false():
    s1 = create_set(source=SET12)
    s2 = create_set(source=SET23)
    assert not s1.issuperset(s2)


def test_set_eq_true():
    s1 = create_set(source=SET12)
    s2 = create_set(source=SET12)
    assert s1 == s2


def test_set_eq_false():
    s1 = create_set(source=SET12)
    s2 = create_set(source=SET123)
    assert s1 != s2
    assert s2 != s1
    s1 = create_set(source=SET12)
    s2 = create_set(source=SET23)
    assert s1 != s2
    assert s2 != s1


def test_set_eq_false_other():
    s = create_set()
    assert s != 7


def test_set_lt_true():
    s1 = create_set(source=SET1)
    s2 = create_set(source=SET12)
    assert s1 < s2


def test_set_lt_false():
    s1 = create_set(source=SET123)
    s2 = create_set(source=SET123)
    assert not s1 < s2


def test_set_le_true():
    s1 = create_set(source=SET123)
    s2 = create_set(source=SET123)
    assert s1 <= s2


def test_set_le_false():
    s1 = create_set(source=SET123)
    s2 = create_set(source=SET123)
    assert not s1 < s2


def test_set_gt_true():
    s1 = create_set(source=SET123)
    s2 = create_set(source=SET23)
    assert s1 > s2


def test_set_gt_false():
    s1 = create_set(source=SET123)
    s2 = create_set(source=SET123)
    assert not s1 > s2


def test_set_ge_true():
    s1 = create_set(source=SET123)
    s2 = create_set(source=SET123)
    assert s1 >= s2


def test_set_ge_false():
    s1 = create_set(source=SET12)
    s2 = create_set(source=SET123)
    assert not s1 >= s2


def test_set_keys():
    m = create_set()
    for k1, k2 in zip(m.keys(), sorted(SET12)):
        assert k1 == k2


def test_set_range():
    m = create_set()
    for i1, i2 in zip(m.range(), sorted(SET12)):
        assert i1 == i2


def test_set_range_lt():
    m = create_set()
    items = list(m.range(lt=K2))
    assert K1 in items
    assert K2 not in items


def test_set_range_le():
    m = create_set()
    items = list(m.range(le=K1))
    assert K1 in items
    assert K2 not in items


def test_set_range_gt():
    m = create_set()
    items = list(m.range(gt=K1))
    assert K1 not in items
    assert K2 in items


def test_set_range_ge():
    m = create_set()
    items = list(m.range(ge=K2))
    assert K1 not in items
    assert K2 in items


def test_set_range_lt_gt():
    m = create_set()
    items = list(m.range(lt=K2, gt=K1))
    assert not items


def test_set_range_le_gt():
    m = create_set()
    items = list(m.range(le=K2, gt=K1))
    assert K1 not in items
    assert K2 in items


def test_set_range_lt_ge():
    m = create_set()
    items = list(m.range(lt=K2, ge=K1))
    assert K1 in items
    assert K2 not in items


def test_set_search_always():
    m = create_set()
    a = Automaton.always()
    items = list(m.search(a))
    for i in SET12:
        assert i in items


def test_set_search_always_complement():
    m = create_set()
    a = Automaton.always().complement()
    items = list(m.search(a))
    assert not items


def test_set_search_never():
    m = create_set()
    a = Automaton.never()
    items = list(m.search(a))
    assert not items


def test_set_search_never_complement():
    m = create_set()
    a = Automaton.never().complement()
    items = list(m.search(a))
    for i in SET12:
        assert i in items


def test_set_search_str():
    m = create_set()
    a = Automaton.str(K1)
    items = list(m.search(a))
    assert K1 in items
    assert K2 not in items


def test_set_search_str_complement():
    m = create_set()
    a = Automaton.str(K1).complement()
    items = list(m.search(a))
    assert K1 not in items
    assert K2 in items


def test_set_search_subsequence():
    m = create_set()
    a = Automaton.subsequence(b"k1")
    items = list(m.search(a))
    assert K1 in items
    assert K2 not in items


def test_set_search_subsequence_complement():
    m = create_set()
    a = Automaton.subsequence(b"k1").complement()
    items = list(m.search(a))
    assert K1 not in items
    assert K2 in items


def test_set_search_starts_with():
    m = create_set(source=SET12O)
    a = Automaton.str(b"key").starts_with()
    items = list(m.search(a))
    assert K1 in items
    assert K2 in items
    assert KO not in items


def test_set_search_starts_with_complement():
    m = create_set(source=SET12O)
    a = Automaton.str(b"key").starts_with().complement()
    items = list(m.search(a))
    assert K1 not in items
    assert K2 not in items
    assert KO in items


def test_set_search_union():
    m = create_set(source=SET12O)
    a1 = Automaton.str(K1)
    a2 = Automaton.str(b"oth").starts_with()
    a = a1.union(a2)
    items = list(m.search(a))
    assert K1 in items
    assert K2 not in items
    assert KO in items


def test_set_search_intersection():
    m = create_set(source=SET123)
    a1 = Automaton.str(K1).complement()
    a2 = Automaton.str(K3).complement()
    a = a1.intersection(a2)
    items = list(m.search(a))
    assert K1 not in items
    assert K2 in items
    assert K3 not in items


def test_set_difference():
    m1 = create_set(source=SET123)
    m2 = create_set(source=SET23)
    m = Set(m1.difference(":memory:", m2))
    items = list(m)
    assert K1 in items
    assert K2 not in items
    assert K3 not in items


def test_set_intersection():
    m1 = create_set(source=SET12)
    m2 = create_set(source=SET23)
    m = Set(m1.intersection(":memory:", m2))
    items = list(m)
    assert K1 not in items
    assert K2 in items
    assert K3 not in items


def test_set_symmetric_difference():
    m1 = create_set(source=SET12)
    m2 = create_set(source=SET23)
    m = Set(m1.symmetric_difference(":memory:", m2))
    items = list(m)
    assert K1 in items
    assert K2 not in items
    assert K3 in items


def test_set_union():
    m1 = create_set(source=SET1)
    m2 = create_set(source=SET23)
    m = Set(m1.union(":memory:", m2))
    assert K1 in m
    assert K2 in m
    assert K3 in m


def test_set_union_multiple():
    m1 = create_set(source=SET12)
    m2 = create_set(source=SET23)
    m = Set(m1.union(":memory:", m2))
    assert K1 in m
    assert K2 in m
    assert K3 in m
