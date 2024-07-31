use fst::{
    automaton::{Automaton, Str, Subsequence},
    set::{OpBuilder, Stream, StreamBuilder},
    IntoStreamer, SetBuilder, Streamer,
};
use ouroboros::self_referencing;
use pyo3::{
    buffer::PyBuffer,
    exceptions::{PyIOError, PyRuntimeError, PyValueError},
    prelude::*,
    types::{PyTuple, PyType},
};
use std::{
    borrow::Cow,
    fs,
    io::{self, BufWriter},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    automaton::{ArcNode, AutomatonGraph},
    buffer::{Buffer, PyBufferRef},
};

const BUFSIZE: usize = 4 * 1024 * 1024;

type PySet = fst::Set<PyBufferRef<u8>>;

type KeyStream<'f> = Box<dyn for<'a> Streamer<'a, Item = &'a [u8]> + Send + 'f>;

#[pyclass(name = "SetIterator")]
#[self_referencing]
struct KeyIterator {
    set: Arc<PySet>,
    str: Vec<u8>,
    #[borrows(set, str)]
    #[not_covariant]
    stream: KeyStream<'this>,
}

#[pymethods]
impl KeyIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<Cow<[u8]>> {
        self.with_stream_mut(|stream| stream.next())
            .map(|key| Cow::from(key.to_vec()))
    }
}

#[pyclass(name = "SetAutomatonIterator")]
#[self_referencing]
struct AutomatonIterator {
    set: Arc<PySet>,
    automaton: ArcNode,
    #[borrows(set, automaton)]
    #[not_covariant]
    stream: Stream<'this, ArcNode>,
}

#[pymethods]
impl AutomatonIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<Cow<[u8]>> {
        self.with_stream_mut(|stream| stream.next())
            .map(|key| Cow::from(key.to_vec()))
    }
}

fn add_range<'m, A: Automaton>(
    mut builder: StreamBuilder<'m, A>,
    ge: Option<&[u8]>,
    gt: Option<&[u8]>,
    le: Option<&[u8]>,
    lt: Option<&[u8]>,
) -> StreamBuilder<'m, A> {
    if let Some(ge) = ge {
        builder = builder.ge(ge);
    }
    if let Some(gt) = gt {
        builder = builder.gt(gt);
    }
    if let Some(le) = le {
        builder = builder.le(le);
    }
    if let Some(lt) = lt {
        builder = builder.lt(lt);
    }
    builder
}

type OpItem<'a> = &'a [u8];

fn fill_from_stream<'f, I, S, W>(stream: I, buf: W) -> PyResult<W>
where
    W: io::Write,
    S: 'f + for<'a> Streamer<'a, Item = OpItem<'a>>,
    I: for<'a> IntoStreamer<'a, Into = S, Item = OpItem<'a>>,
{
    let mut stream = stream.into_stream();
    let mut builder =
        SetBuilder::new(buf).map_err(|err| PyErr::new::<PyRuntimeError, _>(err.to_string()))?;
    while let Some(key) = stream.next() {
        // TODO other options instead of last value
        // unwrap() is OK here, since stream.next() never returns an empty slice
        builder
            .insert(key)
            .map_err(|err| PyErr::new::<PyValueError, _>(err.to_string()))?;
    }
    builder
        .into_inner()
        .map_err(|err| PyErr::new::<PyRuntimeError, _>(err.to_string()))
}

fn build_from_stream<'f, I, S>(path: &Path, stream: I) -> PyResult<Option<Buffer>>
where
    S: 'f + for<'a> Streamer<'a, Item = OpItem<'a>>,
    I: for<'a> IntoStreamer<'a, Into = S, Item = OpItem<'a>>,
{
    if path == Path::new(":memory:") {
        let buf = Vec::with_capacity(10 * (1 << 10));
        let buf = fill_from_stream(stream, buf)?;
        Ok(Some(Buffer::new(buf)))
    } else {
        let wp = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        let writer = BufWriter::with_capacity(BUFSIZE, wp);
        fill_from_stream(stream, writer)?;
        Ok(None)
    }
}

fn fill_from_iterable<W: io::Write>(iterable: &Bound<'_, PyAny>, buf: W) -> PyResult<W> {
    let mut builder =
        SetBuilder::new(buf).map_err(|err| PyErr::new::<PyRuntimeError, _>(err.to_string()))?;
    let iterator = iterable.iter()?;
    for maybe_obj in iterator {
        let obj = maybe_obj?;
        let key = obj.extract::<&[u8]>()?;
        builder
            .insert(key)
            .map_err(|err| PyErr::new::<PyValueError, _>(err.to_string()))?;
    }
    builder
        .into_inner()
        .map_err(|err| PyErr::new::<PyIOError, _>(err.to_string()))
}

fn setvec(first: &Set, tuple: &Bound<'_, PyTuple>) -> PyResult<Vec<Arc<PySet>>> {
    let py = tuple.py();
    let mut sets: Vec<Arc<PySet>> = Vec::with_capacity(tuple.len());
    sets.push(first.inner.clone());
    for other in tuple.iter() {
        let o: Py<Set> = other.extract()?;
        let set = o.borrow(py);
        let set = set.inner.clone();
        sets.push(set);
    }
    Ok(sets)
}

fn opbuilder(sets: &Vec<Arc<PySet>>) -> OpBuilder {
    let mut builder = OpBuilder::new();
    for set in sets {
        builder.push(set.stream());
    }
    builder
}

/// An immutable set of bytes keys, based on finite-state-transducers.
/// Typically uses a fraction of the memory as the builtin set and can be streamed from a file.
///
/// data can be any object that supports the buffer protocol,
/// e.g., Buffer, bytes, memoryview, mmap, etc.
/// Use Map.build to create suitable data.
///
/// Important: data needs to be contiguous.
///
/// To the extent that it's feasible, ducer sets are intended to be direct replacements for the builtin set.
/// For s, o: Set, and k: bytes, the following works as intended:
///
///     k in s
///     s == o
///     len(s)
///     for k in s:
///         pass
///     s.isdisjoint(o)
///     s.issubset(o)
///     s <= o  # subset
///     s < o  # proper subset
///     s.issuperset(o)
///     s >= o  # superset
///     s > o  # proper superset
///
/// Since sets are immutable, the following are **not implemented**:
///
/// - add
/// - clear
/// - difference_update, -=
/// - discard
/// - intersection_update, &=
/// - pop
/// - remove
/// - symmetric_difference_update, ^=
/// - update, |=
///
/// Further, the |, &, -, ^ operators are also not implemented,
/// since it is not possible to specify the storage path.
/// Use Set.union, Set.intersection, Set.difference, and Set.symmetric_difference instead.
#[pyclass(sequence, subclass)]
pub struct Set {
    inner: Arc<PySet>,
}

#[pymethods]
impl Set {
    /// Create a Set from the given data.
    /// data can be any object that supports the buffer protocol,
    /// e.g., bytes, memoryview, mmap, etc.
    ///
    /// Important: data needs to be contiguous.
    #[new]
    fn init(data: &Bound<'_, PyAny>) -> PyResult<Set> {
        let view: PyBuffer<u8> = PyBuffer::get_bound(data)?;
        let slice = PyBufferRef::new(view)?;
        let inner = Arc::new(
            fst::Set::new(slice).map_err(|err| PyErr::new::<PyRuntimeError, _>(err.to_string()))?,
        );
        Ok(Self { inner })
    }

    /// Since sets are immutable, returns self.
    fn copy(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Build a Set from an iterable of bytes
    /// and write it to the given path.
    /// If path is ":memory:", returns a Buffer containing the set data.
    /// Path can be str or Path.
    #[classmethod]
    pub fn build(
        _cls: &Bound<'_, PyType>,
        path: PathBuf,
        iterable: &Bound<'_, PyAny>,
    ) -> PyResult<Option<Buffer>> {
        if path == Path::new(":memory:") {
            let buf = Vec::with_capacity(10 * (1 << 10));
            let w = fill_from_iterable(iterable, buf)?;
            let ret = Buffer::new(w);
            Ok(Some(ret))
        } else {
            let wp = fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(path)?;
            let writer = BufWriter::with_capacity(BUFSIZE, wp);
            fill_from_iterable(iterable, writer)?;
            Ok(None)
        }
    }

    /// Implement iter(self).
    #[allow(clippy::needless_pass_by_value)]
    fn __iter__(&self) -> KeyIterator {
        KeyIteratorBuilder {
            set: self.inner.clone(),
            str: Vec::new(),
            stream_builder: |set, _| Box::new(set.stream()),
        }
        .build()
    }

    /// Returns whether key is in this set.
    fn __contains__(&self, key: &[u8]) -> bool {
        self.inner.contains(key)
    }

    /// Returns number of keys in this set.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Returns this set equals other.
    /// other must be Set.
    fn __eq__(&self, other: &Set) -> bool {
        self.inner.len() == other.inner.len() && {
            let mut s = self.inner.stream();
            let mut o = other.inner.stream();
            loop {
                match (s.next(), o.next()) {
                    (Some(ks), Some(ko)) => {
                        if ks != ko {
                            return false;
                        }
                    }
                    (None, None) => return true,
                    _ => return false,
                }
            }
        }
    }

    /// Returns whether this set is a superset of other.
    /// other must be Set.
    fn __ge__(&self, other: &Set) -> bool {
        self.inner.len() >= other.inner.len() && self.inner.is_superset(other.inner.stream())
    }

    /// Returns whether this set is a proper superset of other.
    /// other must be Set.
    fn __gt__(&self, other: &Set) -> bool {
        self.inner.len() > other.inner.len() && self.inner.is_superset(other.inner.stream())
    }

    /// Returns whether this set is a subset of other.
    /// other must be Set.
    fn __le__(&self, other: &Set) -> bool {
        self.inner.len() <= other.inner.len() && self.inner.is_subset(other.inner.stream())
    }

    /// Returns whether this set is a proper subset of other.
    /// other must be Set.
    fn __lt__(&self, other: &Set) -> bool {
        self.inner.len() < other.inner.len() && self.inner.is_subset(other.inner.stream())
    }

    /// Return True if the set has no elements in common with other.
    /// Sets are disjoint if and only if their intersection is the empty set.
    fn isdisjoint(&self, other: &Set) -> bool {
        self.inner.is_disjoint(other.inner.stream())
    }

    /// Test whether every element in the set is in other.
    fn issubset(&self, other: &Set) -> bool {
        self.inner.is_subset(other.inner.stream())
    }

    /// Test whether every element in other is in the set.
    fn issuperset(&self, other: &Set) -> bool {
        self.inner.is_superset(other.inner.stream())
    }

    /// Iterate over all keys.
    fn keys(&self) -> KeyIterator {
        self.__iter__()
    }

    /// Iterate over all keys that start with str.
    /// Optionally apply range limits
    /// ge (greater than or equal),
    /// gt (greater than),
    /// le (less than or equal),
    /// and lt (less than).
    #[pyo3(signature = (str, ge=None, gt=None, le=None, lt=None))]
    fn starts_with(
        &self,
        str: Vec<u8>,
        ge: Option<&[u8]>,
        gt: Option<&[u8]>,
        le: Option<&[u8]>,
        lt: Option<&[u8]>,
    ) -> KeyIterator {
        KeyIteratorBuilder {
            set: self.inner.clone(),
            str,
            stream_builder: |set, str| {
                Box::new(
                    add_range(
                        set.search(Str::from(str.as_ref()).starts_with()),
                        ge,
                        gt,
                        le,
                        lt,
                    )
                    .into_stream(),
                )
            },
        }
        .build()
    }

    /// Iterate over all keys that contain the subsequence str.
    /// Keys don't need to contain the subsequence consecutively,
    /// e.g., b"bd" will match the key b"abcde".
    /// Optionally apply range limits
    /// ge (greater than or equal),
    /// gt (greater than),
    /// le (less than or equal),
    /// and lt (less than).
    #[pyo3(signature = (str, ge=None, gt=None, le=None, lt=None))]
    fn subsequence(
        &self,
        str: Vec<u8>,
        ge: Option<&[u8]>,
        gt: Option<&[u8]>,
        le: Option<&[u8]>,
        lt: Option<&[u8]>,
    ) -> KeyIterator {
        KeyIteratorBuilder {
            set: self.inner.clone(),
            str,
            stream_builder: |set, str| {
                Box::new(
                    add_range(set.search(Subsequence::from(str.as_ref())), ge, gt, le, lt)
                        .into_stream(),
                )
            },
        }
        .build()
    }

    /// Iterate over all keys that match the given Automaton.
    /// Optionally apply range limits
    /// ge (greater than or equal),
    /// gt (greater than),
    /// le (less than or equal),
    /// and lt (less than).
    #[pyo3(signature = (automaton, ge=None, gt=None, le=None, lt=None))]
    fn search(
        &self,
        automaton: &AutomatonGraph,
        ge: Option<&[u8]>,
        gt: Option<&[u8]>,
        le: Option<&[u8]>,
        lt: Option<&[u8]>,
    ) -> AutomatonIterator {
        AutomatonIteratorBuilder {
            set: self.inner.clone(),
            automaton: automaton.get(),
            stream_builder: |set, automaton| {
                add_range(set.search(automaton.get()), ge, gt, le, lt).into_stream()
            },
        }
        .build()
    }

    /// Iterate over all keys with optional range limits
    /// ge (greater than or equal),
    /// gt (greater than),
    /// le (less than or equal),
    /// and lt (less than).
    /// If no limits are given this is equivalent to iter(self).
    #[pyo3(signature = (ge=None, gt=None, le=None, lt=None))]
    fn range(
        &self,
        ge: Option<&[u8]>,
        gt: Option<&[u8]>,
        le: Option<&[u8]>,
        lt: Option<&[u8]>,
    ) -> KeyIterator {
        KeyIteratorBuilder {
            set: self.inner.clone(),
            str: Vec::new(),
            stream_builder: |set, _| Box::new(add_range(set.range(), ge, gt, le, lt).into_stream()),
        }
        .build()
    }

    /// Build a new set that is the union of self and others.
    /// others must be instances of Set.
    /// If path is ":memory:", returns a Buffer containing the set data
    /// instead of writing to path.
    /// path can be str or Path.
    #[pyo3(signature = (path, *others))]
    #[allow(clippy::needless_pass_by_value)]
    fn union(&self, path: PathBuf, others: &Bound<'_, PyTuple>) -> PyResult<Option<Buffer>> {
        let sets = setvec(self, others)?;
        let stream = opbuilder(&sets).union();
        build_from_stream(&path, stream)
    }

    /// Build a new set that is the intersection of self and others.
    /// others must be instances of Set.
    /// If path is ":memory:", returns a Buffer containing the set data
    /// instead of writing to path.
    /// path can be str or Path.
    #[pyo3(signature = (path, *others))]
    #[allow(clippy::needless_pass_by_value)]
    fn intersection(&self, path: PathBuf, others: &Bound<'_, PyTuple>) -> PyResult<Option<Buffer>> {
        let sets = setvec(self, others)?;
        let stream = opbuilder(&sets).intersection();
        build_from_stream(&path, stream)
    }

    /// Build a new set that is the difference between self and all others,
    /// meaning the resulting set will contain all keys that are in self,
    /// but not in others.
    /// others must be instances of Set.
    /// If path is ":memory:", returns a Buffer containing the set data
    /// instead of writing to path.
    /// path can be str or Path.
    #[pyo3(signature = (path, *others))]
    #[allow(clippy::needless_pass_by_value)]
    fn difference(&self, path: PathBuf, others: &Bound<'_, PyTuple>) -> PyResult<Option<Buffer>> {
        let sets = setvec(self, others)?;
        let stream = opbuilder(&sets).difference();
        build_from_stream(&path, stream)
    }

    /// Build a new set that is the symmetric difference between self and others.
    /// The resulting set will contain all keys that appear an odd number of times, i.e.,
    /// if only one other set is given, it will contain all keys that are in either
    /// self or others, but not in both.
    /// others must be instances of Set.
    /// If path is ":memory:", returns a Buffer containing the set data
    /// instead of writing to path.
    /// path can be str or Path.
    #[pyo3(signature = (path, *others))]
    #[allow(clippy::needless_pass_by_value)]
    fn symmetric_difference(
        &self,
        path: PathBuf,
        others: &Bound<'_, PyTuple>,
    ) -> PyResult<Option<Buffer>> {
        let sets = setvec(self, others)?;
        let stream = opbuilder(&sets).symmetric_difference();
        build_from_stream(&path, stream)
    }
}
