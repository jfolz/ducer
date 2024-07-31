use fst::{
    automaton::{Automaton, Str, Subsequence},
    map::{OpBuilder, Stream, StreamBuilder},
    raw::IndexedValue,
    IntoStreamer, MapBuilder, Streamer,
};
use ouroboros::self_referencing;
use pyo3::{
    buffer::PyBuffer,
    exceptions::{PyIOError, PyKeyError, PyRuntimeError, PyTypeError, PyValueError},
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

use crate::automaton::{ArcNode, AutomatonGraph};
use crate::buffer::{Buffer, PyBufferRef};

type PyMap = fst::Map<PyBufferRef<u8>>;

type ItemStream<'f> = Box<dyn for<'a> Streamer<'a, Item = (&'a [u8], u64)> + Send + 'f>;
type KeyStream<'f> = Box<dyn for<'a> Streamer<'a, Item = &'a [u8]> + Send + 'f>;
type ValueStream<'f> = Box<dyn for<'a> Streamer<'a, Item = u64> + Send + 'f>;

#[pyclass(name = "MapItemIterator")]
#[self_referencing]
struct ItemIterator {
    map: Arc<PyMap>,
    str: Vec<u8>,
    #[borrows(map, str)]
    #[not_covariant]
    stream: ItemStream<'this>,
}

#[pymethods]
impl ItemIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(Cow<[u8]>, u64)> {
        self.with_stream_mut(|stream| stream.next())
            .map(|(key, val)| (Cow::from(key.to_vec()), val))
    }
}

#[pyclass(name = "MapKeyIterator")]
#[self_referencing]
struct KeyIterator {
    map: Arc<PyMap>,
    str: Vec<u8>,
    #[borrows(map, str)]
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

#[pyclass(name = "MapValueIterator")]
#[self_referencing]
struct ValueIterator {
    map: Arc<PyMap>,
    str: Vec<u8>,
    #[borrows(map, str)]
    #[not_covariant]
    stream: ValueStream<'this>,
}

#[pymethods]
impl ValueIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<u64> {
        self.with_stream_mut(|stream| stream.next())
    }
}

#[pyclass(name = "MapAutomatonIterator")]
#[self_referencing]
struct AutomatonIterator {
    map: Arc<PyMap>,
    automaton: ArcNode,
    #[borrows(map, automaton)]
    #[not_covariant]
    stream: Stream<'this, ArcNode>,
}

#[pymethods]
impl AutomatonIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(Cow<[u8]>, u64)> {
        self.with_stream_mut(|stream| stream.next())
            .map(|(key, val)| (Cow::from(key.to_vec()), val))
    }
}

const BUFSIZE: usize = 4 * 1024 * 1024;

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

type OpItem<'a> = (&'a [u8], &'a [IndexedValue]);

fn fill_from_stream<'f, I, S, F, W>(stream: I, select: F, buf: W) -> PyResult<W>
where
    W: io::Write,
    S: 'f + for<'a> Streamer<'a, Item = OpItem<'a>>,
    I: for<'a> IntoStreamer<'a, Into = S, Item = OpItem<'a>>,
    F: Fn(&[IndexedValue]) -> u64,
{
    let mut stream = stream.into_stream();
    let mut builder =
        MapBuilder::new(buf).map_err(|err| PyErr::new::<PyRuntimeError, _>(err.to_string()))?;
    while let Some((key, posval)) = stream.next() {
        // TODO other options instead of last value
        // unwrap() is OK here, since stream.next() never returns an empty slice
        builder
            .insert(key, select(posval))
            .map_err(|err| PyErr::new::<PyValueError, _>(err.to_string()))?;
    }
    builder
        .into_inner()
        .map_err(|err| PyErr::new::<PyRuntimeError, _>(err.to_string()))
}

fn build_from_stream<'f, I, S, F>(path: &Path, stream: I, select: F) -> PyResult<Option<Buffer>>
where
    S: 'f + for<'a> Streamer<'a, Item = OpItem<'a>>,
    I: for<'a> IntoStreamer<'a, Into = S, Item = OpItem<'a>>,
    F: Fn(&[IndexedValue]) -> u64,
{
    if path == Path::new(":memory:") {
        let buf = Vec::with_capacity(10 * (1 << 10));
        let buf = fill_from_stream(stream, select, buf)?;
        Ok(Some(Buffer::new(buf)))
    } else {
        let wp = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        let writer = BufWriter::with_capacity(BUFSIZE, wp);
        fill_from_stream(stream, select, writer)?;
        Ok(None)
    }
}

#[inline]
fn insert_pyobject<W: io::Write>(
    obj: &Bound<'_, PyAny>,
    builder: &mut MapBuilder<W>,
) -> PyResult<()> {
    let item0;
    let (key, val) = if let Ok(tuple) = obj.downcast::<PyTuple>() {
        let items = tuple.as_slice();
        if items.len() != 2 {
            return Err(PyErr::new::<PyTypeError, _>(
                "map items must be sequences with length 2, e.g. tuple (key: bytes, value: int)",
            ));
        }
        let key = items[0].extract::<&[u8]>()?;
        let val = items[1].extract::<u64>()?;
        (key, val)
    } else {
        if obj.len()? != 2 {
            return Err(PyErr::new::<PyTypeError, _>(
                "map items must be sequences with length 2, e.g. tuple (key: bytes, value: int)",
            ));
        }
        item0 = obj.get_item(0)?;
        let key = item0.extract::<&[u8]>()?;
        let val = obj.get_item(1)?.extract::<u64>()?;
        (key, val)
    };
    builder
        .insert(key, val)
        .map_err(|err| PyErr::new::<PyValueError, _>(err.to_string()))
}

fn fill_from_iterable<W: io::Write>(iterable: &Bound<'_, PyAny>, buf: W) -> PyResult<W> {
    let mut builder =
        MapBuilder::new(buf).map_err(|err| PyErr::new::<PyRuntimeError, _>(err.to_string()))?;
    let iterator = iterable.iter()?;
    for maybe_obj in iterator {
        let obj = maybe_obj?;
        insert_pyobject(&obj, &mut builder)?;
    }
    builder
        .into_inner()
        .map_err(|err| PyErr::new::<PyIOError, _>(err.to_string()))
}

fn mapvec(first: &Map, tuple: &Bound<'_, PyTuple>) -> PyResult<Vec<Arc<PyMap>>> {
    let py = tuple.py();
    let mut maps: Vec<Arc<PyMap>> = Vec::with_capacity(tuple.len());
    maps.push(first.inner.clone());
    for other in tuple.iter() {
        let o: Py<Map> = other.extract()?;
        let map = o.borrow(py);
        let map = map.inner.clone();
        maps.push(map);
    }
    Ok(maps)
}

fn opbuilder(maps: &Vec<Arc<PyMap>>) -> OpBuilder {
    let mut builder = OpBuilder::new();
    for map in maps {
        builder.push(map.stream());
    }
    builder
}

/// Conflict resolution strategies for set operations on maps.
#[pyclass(eq, eq_int)]
#[derive(PartialEq, Clone)]
pub enum Op {
    /// Select first value.
    First,
    /// Select middle value, i.e., values[len // 2].
    Mid,
    /// Select last value.
    Last,
    /// Select minimum.
    Min,
    /// Select maximum.
    Max,
    /// Select average, i.e., sum(values) // len.
    Avg,
    /// Select median, i.e., with values = sorted(values) and mid = len // 2
    /// for odd length values[mid],
    /// and (values[mid-1] + values[mid]) // 2 for even length.
    Median,
}

#[allow(clippy::needless_pass_by_value)]
fn select_value(sf: Op, posval: &[IndexedValue]) -> u64 {
    match sf {
        Op::First => posval.first().unwrap().value,
        Op::Mid => posval[posval.len() / 2].value,
        Op::Last => posval.last().unwrap().value,
        Op::Min => posval.iter().map(|i| i.value).min().unwrap(),
        Op::Max => posval.iter().map(|i| i.value).max().unwrap(),
        Op::Avg => posval.iter().map(|i| i.value).sum::<u64>() / (posval.len() as u64),
        Op::Median => {
            let mut values: Vec<u64> = posval.iter().map(|i| i.value).collect();
            let n = values.len();
            let mid = n / 2;
            let (lesser, median, _) = values.select_nth_unstable(mid);
            if n % 2 == 1 {
                // odd length
                *median
            } else {
                // even length
                (*median + lesser.iter().max().unwrap()) / 2
            }
        }
    }
}

/// An immutable map of bytes keys and non-negative integers, based on finite-state-transducers.
/// Typically uses a fraction of the memory as the builtin dict and can be streamed from a file.
///
/// data can be any object that supports the buffer protocol,
/// e.g., Buffer, bytes, memoryview, mmap, etc.
/// Use Map.build to create suitable data.
///
/// Important: data needs to be contiguous.
///
/// To the extent that it's feasible, ducer maps are intended to be direct replacements for the builtin dict.
/// For m, o: Map and k: bytes, the following works as intended:
///
///     k in m
///     m == o
///     m[k]
///     m.get(k)
///     m.get(k, 42)
///     len(m)
///     for k in m:
///         pass
///     for k in m.keys():
///         pass
///     for v in m.values():
///         pass
///     for k, v in m.items():
///         pass
///
/// Since maps are immutable, the following are not implemented:
///
/// - clear
/// - fromkeys
/// - pop
/// - popitem
/// - setdefault
/// - update, |=
///
/// Further, the |, &, -, ^ operators are also not implemented,
/// since it is not possible to specify the storage path.
/// Use Map.union, Map.intersection, Map.difference, and Map.symmetric_difference instead.
#[pyclass(mapping, subclass)]
pub struct Map {
    inner: Arc<PyMap>,
}

#[pymethods]
impl Map {
    /// Create a Map from the given data.
    /// data can be any object that supports the buffer protocol,
    /// e.g., bytes, memoryview, mmap, etc.
    ///
    /// Important: data needs to be contiguous.
    #[new]
    fn init(data: &Bound<'_, PyAny>) -> PyResult<Map> {
        let view: PyBuffer<u8> = PyBuffer::get_bound(data)?;
        let slice = PyBufferRef::new(view)?;
        let inner = Arc::new(
            fst::Map::new(slice).map_err(|err| PyErr::new::<PyRuntimeError, _>(err.to_string()))?,
        );
        Ok(Self { inner })
    }

    /// Since maps are immutable, returns self.
    fn copy(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    /// Build a map from an iterable of items (key: bytes, value: int)
    /// and write it to the given path.
    /// If path is ":memory:", returns a Buffer containing the map data.
    /// path can be str or Path.
    ///
    /// Hint:
    ///     Items can really be any sequence of length 2, but building from tuple is fastest.
    ///     However, avoid converting items in Python for best performance.
    ///     Ideally, create tuples directly, e.g., if using msgpack,
    ///     set use_list=False for msgpack.unpackb or msgpack.Unpacker.
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
    /// Like the builtin dict, only keys are returned.
    fn __iter__(&self) -> KeyIterator {
        self.keys()
    }

    /// Implement self[key].
    fn __getitem__(&self, key: &[u8]) -> PyResult<u64> {
        if let Some(val) = self.inner.get(key) {
            Ok(val)
        } else {
            Err(PyErr::new::<PyKeyError, _>(Cow::from(key.to_owned())))
        }
    }

    /// Returns whether this map contains key.
    fn __contains__(&self, key: &[u8]) -> bool {
        self.inner.contains_key(key)
    }

    /// Returns number of items in this map.
    fn __len__(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether this map equals other.
    /// Other must be Map.
    fn __eq__(&self, other: &Map) -> bool {
        self.inner.len() == other.inner.len() && {
            let mut s = self.inner.stream();
            let mut o = other.inner.stream();
            loop {
                match (s.next(), o.next()) {
                    (Some(s), Some(o)) => {
                        if s != o {
                            return false;
                        }
                    }
                    (None, None) => return true,
                    _ => return false,
                }
            }
        }
    }

    /// Returns the given key if present, default otherwise.
    #[pyo3(signature=(key, default=None))]
    fn get(&self, key: &[u8], default: Option<u64>) -> Option<u64> {
        self.inner.get(key).or(default)
    }

    /// Iterate over all key-value items.
    fn items(&self) -> ItemIterator {
        ItemIteratorBuilder {
            map: self.inner.clone(),
            str: Vec::new(),
            stream_builder: |map, _| Box::new(map.stream()),
        }
        .build()
    }

    /// Iterate over all keys.
    fn keys(&self) -> KeyIterator {
        KeyIteratorBuilder {
            map: self.inner.clone(),
            str: Vec::new(),
            stream_builder: |map, _| Box::new(map.keys()),
        }
        .build()
    }

    /// Iterate over all values.
    fn values(&self) -> ValueIterator {
        ValueIteratorBuilder {
            map: self.inner.clone(),
            str: Vec::new(),
            stream_builder: |map, _| Box::new(map.values()),
        }
        .build()
    }

    /// Iterate over all key-value items whose key starts with str.
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
    ) -> ItemIterator {
        ItemIteratorBuilder {
            map: self.inner.clone(),
            str,
            stream_builder: |map, str| {
                Box::new(
                    add_range(
                        map.search(Str::from(str.as_ref()).starts_with()),
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

    /// Iterate over all key-value items whose key contain the subsequence str.
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
    ) -> ItemIterator {
        ItemIteratorBuilder {
            map: self.inner.clone(),
            str,
            stream_builder: |map, str| {
                Box::new(
                    add_range(map.search(Subsequence::from(str.as_ref())), ge, gt, le, lt)
                        .into_stream(),
                )
            },
        }
        .build()
    }

    /// Iterate over all key-value items whose key matches the given Automaton.
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
            map: self.inner.clone(),
            automaton: automaton.get(),
            stream_builder: |map, automaton| {
                add_range(map.search(automaton.get()), ge, gt, le, lt).into_stream()
            },
        }
        .build()
    }

    /// Iterate over all key-value items with optional range limits for the key
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
    ) -> ItemIterator {
        ItemIteratorBuilder {
            map: self.inner.clone(),
            str: Vec::new(),
            stream_builder: |map, _| Box::new(add_range(map.range(), ge, gt, le, lt).into_stream()),
        }
        .build()
    }

    /// Build a new map that is the union of self and others.
    /// others must be instances of Map.
    /// select specifies how conflicts are resolved if keys are
    /// present more than once.
    /// If path is ":memory:", returns a Buffer containing the map data
    /// instead of writing to path.
    /// path can be str or Path.
    #[pyo3(signature = (path, *others, select=Op::Last))]
    #[allow(clippy::needless_pass_by_value)]
    fn union(
        &self,
        path: PathBuf,
        others: &Bound<'_, PyTuple>,
        select: Op,
    ) -> PyResult<Option<Buffer>> {
        let maps = mapvec(self, others)?;
        let stream = opbuilder(&maps).union();
        build_from_stream(&path, stream, |posval| select_value(select.clone(), posval))
    }

    /// Build a new map that is the intersection of self and others.
    /// others must be instances of Map.
    /// select specifies how conflicts are resolved if keys are
    /// present more than once.
    /// If path is ":memory:", returns a Buffer containing the map data
    /// instead of writing to path.
    /// path can be str or Path.
    #[pyo3(signature = (path, *others, select=Op::Last))]
    #[allow(clippy::needless_pass_by_value)]
    fn intersection(
        &self,
        path: PathBuf,
        others: &Bound<'_, PyTuple>,
        select: Op,
    ) -> PyResult<Option<Buffer>> {
        let maps = mapvec(self, others)?;
        let stream = opbuilder(&maps).intersection();
        build_from_stream(&path, stream, |posval| select_value(select.clone(), posval))
    }

    /// Build a new map that is the difference between self and all others,
    /// meaning the resulting map will contain all keys that are in self,
    /// but not in others.
    /// others must be instances of Map.
    /// select specifies how conflicts are resolved if keys are
    /// present more than once.
    /// If path is ":memory:", returns a Buffer containing the map data
    /// instead of writing to path.
    /// path can be str or Path.
    #[pyo3(signature = (path, *others, select=Op::Last))]
    #[allow(clippy::needless_pass_by_value)]
    fn difference(
        &self,
        path: PathBuf,
        others: &Bound<'_, PyTuple>,
        select: Op,
    ) -> PyResult<Option<Buffer>> {
        let maps = mapvec(self, others)?;
        let stream = opbuilder(&maps).difference();
        build_from_stream(&path, stream, |posval| select_value(select.clone(), posval))
    }

    /// Build a new map that is the symmetric difference between self and others.
    /// The resulting map will contain all keys that appear an odd number of times, i.e.,
    /// if only one other map is given, it will contain all keys that are in either
    /// self or others, but not in both.
    /// others must be instances of Map.
    /// select specifies how conflicts are resolved if keys are
    /// present more than once.
    /// If path is ":memory:", returns a Buffer containing the map data
    /// instead of writing to path.
    /// path can be str or Path.
    #[pyo3(signature = (path, *others, select=Op::Last))]
    #[allow(clippy::needless_pass_by_value)]
    fn symmetric_difference(
        &self,
        path: PathBuf,
        others: &Bound<'_, PyTuple>,
        select: Op,
    ) -> PyResult<Option<Buffer>> {
        let maps = mapvec(self, others)?;
        let stream = opbuilder(&maps).symmetric_difference();
        build_from_stream(&path, stream, |posval| select_value(select.clone(), posval))
    }
}
