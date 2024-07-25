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
    str: String,
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

fn setvec(tuple: &Bound<'_, PyTuple>) -> PyResult<Vec<Arc<PySet>>> {
    let py = tuple.py();
    let mut maps: Vec<Arc<PySet>> = Vec::with_capacity(tuple.len());
    for other in tuple.iter() {
        let o: Py<Set> = other.extract()?;
        let set = o.borrow(py);
        let set = set.inner.clone();
        maps.push(set);
    }
    Ok(maps)
}

fn opbuilder(maps: &Vec<Arc<PySet>>) -> OpBuilder {
    let mut builder = OpBuilder::new();
    for map in maps {
        builder.push(map.stream());
    }
    builder
}

#[pyclass(sequence)]
pub struct Set {
    inner: Arc<PySet>,
}

#[pymethods]
impl Set {
    /// Create a `Set` from the given data.
    /// `data` can be any object that supports the buffer protocol,
    /// e.g., `bytes`, `memoryview`, `mmap`, etc.
    /// Important: `data` needs to be contiguous.
    #[new]
    fn init(data: &Bound<'_, PyAny>) -> PyResult<Set> {
        let view: PyBuffer<u8> = PyBuffer::get_bound(data)?;
        let slice = PyBufferRef::new(view)?;
        let inner = Arc::new(
            fst::Set::new(slice).map_err(|err| PyErr::new::<PyRuntimeError, _>(err.to_string()))?,
        );
        Ok(Self { inner })
    }

    /// Since `Set` is stateless, returns self.
    fn copy<'a>(slf: PyRef<'a, Self>) -> PyRef<'a, Self> {
        slf
    }

    /// Build a Set from an iterable of `bytes`
    /// and write it to the given path.
    /// If path is `:memory:`, returns a `Buffer` containing the set data.
    /// Path can be `str` or `pathlib.Path`.
    #[classmethod]
    pub fn build(
        _cls: &Bound<'_, PyType>,
        iterable: &Bound<'_, PyAny>,
        path: PathBuf,
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

    #[allow(clippy::needless_pass_by_value)]
    fn __iter__(&self) -> KeyIterator {
        KeyIteratorBuilder {
            set: self.inner.clone(),
            str: String::new(),
            stream_builder: |set, _| Box::new(set.stream()),
        }
        .build()
    }

    fn __contains__(&self, key: &[u8]) -> bool {
        self.inner.contains(key)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

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

    fn __ge__(&self, other: &Set) -> bool {
        self.inner.len() >= other.inner.len() && self.inner.is_superset(other.inner.stream())
    }

    fn __gt__(&self, other: &Set) -> bool {
        self.inner.len() > other.inner.len() && self.inner.is_superset(other.inner.stream())
    }

    fn __le__(&self, other: &Set) -> bool {
        self.inner.len() <= other.inner.len() && self.inner.is_subset(other.inner.stream())
    }

    fn __lt__(&self, other: &Set) -> bool {
        self.inner.len() < other.inner.len() && self.inner.is_subset(other.inner.stream())
    }

    fn isdisjoint(&self, other: &Set) -> bool {
        self.inner.is_disjoint(other.inner.stream())
    }

    fn issubset(&self, other: &Set) -> bool {
        self.inner.is_subset(other.inner.stream())
    }

    fn issuperset(&self, other: &Set) -> bool {
        self.inner.is_superset(other.inner.stream())
    }

    fn keys(&self) -> KeyIterator {
        self.__iter__()
    }

    #[pyo3(signature = (str, ge=None, gt=None, le=None, lt=None))]
    fn starts_with(
        &self,
        str: String,
        ge: Option<&[u8]>,
        gt: Option<&[u8]>,
        le: Option<&[u8]>,
        lt: Option<&[u8]>,
    ) -> KeyIterator {
        KeyIteratorBuilder {
            set: self.inner.clone(),
            str,
            stream_builder: |map, str| {
                Box::new(
                    add_range(map.search(Str::new(str).starts_with()), ge, gt, le, lt)
                        .into_stream(),
                )
            },
        }
        .build()
    }

    #[pyo3(signature = (str, ge=None, gt=None, le=None, lt=None))]
    fn subsequence(
        &self,
        str: String,
        ge: Option<&[u8]>,
        gt: Option<&[u8]>,
        le: Option<&[u8]>,
        lt: Option<&[u8]>,
    ) -> KeyIterator {
        KeyIteratorBuilder {
            set: self.inner.clone(),
            str,
            stream_builder: |map, str| {
                Box::new(add_range(map.search(Subsequence::new(str)), ge, gt, le, lt).into_stream())
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
            set: self.inner.clone(),
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
    ) -> KeyIterator {
        KeyIteratorBuilder {
            set: self.inner.clone(),
            str: String::new(),
            stream_builder: |map, _| Box::new(add_range(map.range(), ge, gt, le, lt).into_stream()),
        }
        .build()
    }

    #[classmethod]
    #[pyo3(signature = (path, *maps))]
    #[allow(clippy::needless_pass_by_value)]
    fn union(
        _cls: &Bound<'_, PyType>,
        path: PathBuf,
        maps: &Bound<'_, PyTuple>,
    ) -> PyResult<Option<Buffer>> {
        let maps = setvec(maps)?;
        let stream = opbuilder(&maps).union();
        build_from_stream(&path, stream)
    }

    #[classmethod]
    #[pyo3(signature = (path, *maps))]
    #[allow(clippy::needless_pass_by_value)]
    fn intersection(
        _cls: &Bound<'_, PyType>,
        path: PathBuf,
        maps: &Bound<'_, PyTuple>,
    ) -> PyResult<Option<Buffer>> {
        let maps = setvec(maps)?;
        let stream = opbuilder(&maps).intersection();
        build_from_stream(&path, stream)
    }

    #[classmethod]
    #[pyo3(signature = (path, *maps))]
    #[allow(clippy::needless_pass_by_value)]
    fn difference(
        _cls: &Bound<'_, PyType>,
        path: PathBuf,
        maps: &Bound<'_, PyTuple>,
    ) -> PyResult<Option<Buffer>> {
        let maps = setvec(maps)?;
        let stream = opbuilder(&maps).difference();
        build_from_stream(&path, stream)
    }

    #[classmethod]
    #[pyo3(signature = (path, *maps))]
    #[allow(clippy::needless_pass_by_value)]
    fn symmetric_difference(
        _cls: &Bound<'_, PyType>,
        path: PathBuf,
        maps: &Bound<'_, PyTuple>,
    ) -> PyResult<Option<Buffer>> {
        let maps = setvec(maps)?;
        let stream = opbuilder(&maps).symmetric_difference();
        build_from_stream(&path, stream)
    }
}
