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

#[pyclass(eq, eq_int)]
#[derive(PartialEq, Clone)]
pub enum Op {
    First,
    Mid,
    Last,
    Min,
    Max,
    Avg,
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

#[pyclass(mapping)]
pub struct Map {
    inner: Arc<PyMap>,
}

#[pymethods]
impl Map {
    /// Create a `Map` from the given data.
    /// `data` can be any object that supports the buffer protocol,
    /// e.g., `bytes`, `memoryview`, `mmap`, etc.
    /// Important: `data` needs to be contiguous.
    #[new]
    fn init(data: &Bound<'_, PyAny>) -> PyResult<Map> {
        let view: PyBuffer<u8> = PyBuffer::get_bound(data)?;
        let slice = PyBufferRef::new(view)?;
        let inner = Arc::new(
            fst::Map::new(slice).map_err(|err| PyErr::new::<PyRuntimeError, _>(err.to_string()))?,
        );
        Ok(Self { inner })
    }

    /// Since `Map` is stateless, returns self.
    fn copy<'a>(slf: PyRef<'a, Self>) -> PyRef<'a, Self> {
        slf
    }

    /// Build a Map from an iterable of items `(key: bytes, value: int)`
    /// and write it to the given path.
    /// If path is `:memory:`, returns a `Buffer` containing the map data.
    /// Path can be `str` or `pathlib.Path`.
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

    fn __iter__(&self) -> KeyIterator {
        self.keys()
    }

    fn __getitem__(&self, key: &[u8]) -> PyResult<u64> {
        if let Some(val) = self.inner.get(key) {
            Ok(val)
        } else {
            Err(PyErr::new::<PyKeyError, _>(Cow::from(key.to_owned())))
        }
    }

    fn __contains__(&self, key: &[u8]) -> bool {
        self.inner.contains_key(key)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

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

    #[pyo3(signature=(key, default=None))]
    fn get(&self, key: &[u8], default: Option<u64>) -> Option<u64> {
        self.inner.get(key).or_else(|| default)
    }

    fn items(&self) -> ItemIterator {
        ItemIteratorBuilder {
            map: self.inner.clone(),
            str: Vec::new(),
            stream_builder: |map, _| Box::new(map.stream()),
        }
        .build()
    }

    fn keys(&self) -> KeyIterator {
        KeyIteratorBuilder {
            map: self.inner.clone(),
            str: Vec::new(),
            stream_builder: |map, _| Box::new(map.keys()),
        }
        .build()
    }

    fn values(&self) -> ValueIterator {
        ValueIteratorBuilder {
            map: self.inner.clone(),
            str: Vec::new(),
            stream_builder: |map, _| Box::new(map.values()),
        }
        .build()
    }

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

    #[pyo3(signature = (path, *maps, select=Op::Last))]
    #[allow(clippy::needless_pass_by_value)]
    fn union(
        &self,
        path: PathBuf,
        maps: &Bound<'_, PyTuple>,
        select: Op,
    ) -> PyResult<Option<Buffer>> {
        let maps = mapvec(self, maps)?;
        let stream = opbuilder(&maps).union();
        build_from_stream(&path, stream, |posval| select_value(select.clone(), posval))
    }

    #[pyo3(signature = (path, *maps, select=Op::Last))]
    #[allow(clippy::needless_pass_by_value)]
    fn intersection(
        &self,
        path: PathBuf,
        maps: &Bound<'_, PyTuple>,
        select: Op,
    ) -> PyResult<Option<Buffer>> {
        let maps = mapvec(self, maps)?;
        let stream = opbuilder(&maps).intersection();
        build_from_stream(&path, stream, |posval| select_value(select.clone(), posval))
    }

    #[pyo3(signature = (path, *maps, select=Op::Last))]
    #[allow(clippy::needless_pass_by_value)]
    fn difference(
        &self,
        path: PathBuf,
        maps: &Bound<'_, PyTuple>,
        select: Op,
    ) -> PyResult<Option<Buffer>> {
        let maps = mapvec(self, maps)?;
        let stream = opbuilder(&maps).difference();
        build_from_stream(&path, stream, |posval| select_value(select.clone(), posval))
    }

    #[pyo3(signature = (path, *maps, select=Op::Last))]
    #[allow(clippy::needless_pass_by_value)]
    fn symmetric_difference(
        &self,
        path: PathBuf,
        maps: &Bound<'_, PyTuple>,
        select: Op,
    ) -> PyResult<Option<Buffer>> {
        let maps = mapvec(self, maps)?;
        let stream = opbuilder(&maps).symmetric_difference();
        build_from_stream(&path, stream, |posval| select_value(select.clone(), posval))
    }
}
