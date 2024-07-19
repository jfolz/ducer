use fst::{
    automaton::{Automaton, Str, Subsequence},
    map::Stream,
    IntoStreamer, Streamer,
};
use ouroboros::self_referencing;
use pyo3::{buffer::PyBuffer, prelude::*, types::PyTuple};
use std::{
    borrow::Cow,
    fs,
    io::{self, BufWriter},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::automaton::{ArcNode, AutomatonGraph};
use crate::buffer::{Buffer, PyBufferRef};

type ItemStream<'f> = Box<dyn for<'a> Streamer<'a, Item = (&'a [u8], u64)> + Send + 'f>;
type KeyStream<'f> = Box<dyn for<'a> Streamer<'a, Item = &'a [u8]> + Send + 'f>;
type ValueStream<'f> = Box<dyn for<'a> Streamer<'a, Item = u64> + Send + 'f>;

#[pyclass]
#[self_referencing]
struct MapItemIterator {
    map: Arc<fst::Map<PyBufferRef<u8>>>,
    str: String,
    #[borrows(map, str)]
    #[not_covariant]
    stream: ItemStream<'this>,
}

#[pymethods]
impl MapItemIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(Cow<[u8]>, u64)> {
        self.with_stream_mut(|stream| stream.next())
            .map(|(key, val)| (Cow::from(key.to_vec()), val))
    }
}

#[pyclass]
#[self_referencing]
struct MapKeyIterator {
    map: Arc<fst::Map<PyBufferRef<u8>>>,
    str: String,
    #[borrows(map, str)]
    #[not_covariant]
    stream: KeyStream<'this>,
}

#[pymethods]
impl MapKeyIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<Cow<[u8]>> {
        self.with_stream_mut(|stream| stream.next())
            .map(|key| Cow::from(key.to_vec()))
    }
}

#[pyclass]
#[self_referencing]
struct MapValueIterator {
    map: Arc<fst::Map<PyBufferRef<u8>>>,
    str: String,
    #[borrows(map, str)]
    #[not_covariant]
    stream: ValueStream<'this>,
}

#[pymethods]
impl MapValueIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<u64> {
        self.with_stream_mut(|stream| stream.next())
    }
}

#[pyclass]
#[self_referencing]
struct MapAutomatonIterator {
    map: Arc<fst::Map<PyBufferRef<u8>>>,
    automaton: ArcNode,
    #[borrows(map, automaton)]
    #[not_covariant]
    stream: Stream<'this, ArcNode>,
}

#[pymethods]
impl MapAutomatonIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(Cow<[u8]>, u64)> {
        self.with_stream_mut(|stream| stream.next())
            .map(|(key, val)| (Cow::from(key.to_vec()), val))
    }
}

const BUFSIZE: usize = 4 * 1024 * 1024;

#[pyclass(mapping)]
pub struct Map {
    inner: Arc<fst::Map<PyBufferRef<u8>>>,
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
        let inner =
            Arc::new(fst::Map::new(slice).map_err(|err| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string())
            })?);
        Ok(Self { inner })
    }

    fn __iter__(&self) -> MapKeyIterator {
        self.keys()
    }

    fn __getitem__(&self, key: &[u8]) -> Option<u64> {
        self.inner.get(key)
    }

    fn __contains__(&self, key: &[u8]) -> bool {
        self.inner.contains_key(key)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }

    fn items(&self) -> MapItemIterator {
        MapItemIteratorBuilder {
            map: self.inner.clone(),
            str: String::new(),
            stream_builder: |map, _| Box::new(map.stream()),
        }
        .build()
    }

    fn keys(&self) -> MapKeyIterator {
        MapKeyIteratorBuilder {
            map: self.inner.clone(),
            str: String::new(),
            stream_builder: |map, _| Box::new(map.keys()),
        }
        .build()
    }

    fn values(&self) -> MapValueIterator {
        MapValueIteratorBuilder {
            map: self.inner.clone(),
            str: String::new(),
            stream_builder: |map, _| Box::new(map.values()),
        }
        .build()
    }

    fn starts_with(&self, str: String) -> MapItemIterator {
        MapItemIteratorBuilder {
            map: self.inner.clone(),
            str,
            stream_builder: |map, str| {
                Box::new(map.search(Str::new(str).starts_with()).into_stream())
            },
        }
        .build()
    }

    fn subsequence(&self, str: String) -> MapItemIterator {
        MapItemIteratorBuilder {
            map: self.inner.clone(),
            str,
            stream_builder: |map, str| Box::new(map.search(Subsequence::new(str)).into_stream()),
        }
        .build()
    }

    fn search(&self, automaton: &AutomatonGraph) -> MapAutomatonIterator {
        MapAutomatonIteratorBuilder {
            map: self.inner.clone(),
            automaton: automaton.get(),
            stream_builder: |map, automaton| map.search(automaton.get()).into_stream(),
        }
        .build()
    }
}

#[allow(clippy::module_name_repetitions)]
fn fill_map<W: io::Write>(
    iterable: &Bound<'_, PyAny>,
    mut builder: fst::MapBuilder<W>,
) -> PyResult<W> {
    let iterator = iterable.iter()?;
    for maybe_obj in iterator {
        let obj = maybe_obj?;

        let item0;
        let (key, val) = if let Ok(tuple) = obj.downcast::<PyTuple>() {
            let items = tuple.as_slice();
            if items.len() != 2 {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "map items must be sequences with length 2, e.g. tuple (key: bytes, value: int)",
                ));
            }
            let key = items[0].extract::<&[u8]>()?;
            let val = items[1].extract::<u64>()?;
            (key, val)
        } else {
            if obj.len()? != 2 {
                return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
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
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string()))?;
    }
    builder
        .into_inner()
        .map_err(|err| PyErr::new::<pyo3::exceptions::PyIOError, _>(err.to_string()))
}

/// Build a Map from an iterable of items `(key: bytes, value: int)`
/// and write it to the given path.
/// If path is `:memory:`, returns a `Buffer` containing the map data.
#[pyfunction]
#[allow(clippy::module_name_repetitions)]
pub fn build_map(iterable: &Bound<'_, PyAny>, path: PathBuf) -> PyResult<Option<Buffer>> {
    if path == Path::new(":memory:") {
        let buf = Vec::with_capacity(10 * (1 << 10));
        let builder = fst::MapBuilder::new(buf)
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string()))?;
        let w = fill_map(iterable, builder)?;
        let ret = Buffer::new(w);
        Ok(Some(ret))
    } else {
        let wp = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        let writer = BufWriter::with_capacity(BUFSIZE, wp);
        fill_map(
            iterable,
            fst::MapBuilder::new(writer)
                .map_err(|err| PyErr::new::<pyo3::exceptions::PyTypeError, _>(err.to_string()))?,
        )?;
        Ok(None)
    }
}
