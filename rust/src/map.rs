use fst::{
    automaton::{AlwaysMatch, Automaton, StartsWith, Str, Subsequence},
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

use crate::automaton::{ArcAutomatonGraphNode, AutomatonGraph};
use crate::buffer::{Buffer, PyBufferRef};

macro_rules! define_iterators {
    ($($name:ident $generic:ty),* $(,)?) => {
        $(
            #[pyclass]
            #[self_referencing]
            struct $name {
                map: Arc<fst::Map<PyBufferRef<u8>>>,
                str: String,
                #[borrows(map, str)]
                #[not_covariant]
                stream: Stream<'this, $generic>,
            }

            #[pymethods]
            impl $name {
                fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
                    slf
                }

                fn __next__(&mut self) -> Option<(Cow<[u8]>, u64)> {
                    match &self.with_stream_mut(|stream| stream.next()) {
                        Some((key, val)) => {
                            Some((Cow::from(key.to_vec()), *val))
                        }
                        None => None,
                    }
                }
            }
        )*
    };
}

// Use the macro to define multiple structs
define_iterators!(
    MapIterator AlwaysMatch,
    MapStartsWithIterator StartsWith<Str<'this>>,
    MapSubsequenceIterator Subsequence<'this>,
);

#[pyclass]
#[self_referencing]
struct MapAutomatonIterator {
    map: Arc<fst::Map<PyBufferRef<u8>>>,
    automaton: ArcAutomatonGraphNode,
    #[borrows(map, automaton)]
    #[not_covariant]
    stream: Stream<'this, ArcAutomatonGraphNode>,
}

#[pymethods]
impl MapAutomatonIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(&mut self) -> Option<(Cow<[u8]>, u64)> {
        match &self.with_stream_mut(|stream| stream.next()) {
            Some((key, val)) => Some((Cow::from(key.to_vec()), *val)),
            None => None,
        }
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
    fn init<'py>(data: &Bound<'py, PyAny>) -> PyResult<Map> {
        let view: PyBuffer<u8> = PyBuffer::get_bound(data)?;
        let slice = PyBufferRef::new(view)?;
        let inner =
            Arc::new(fst::Map::new(slice).map_err(|err| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string())
            })?);
        Ok(Self { inner })
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<MapIterator>> {
        let iter = MapIteratorBuilder {
            map: slf.inner.clone(),
            str: "".to_owned(),
            stream_builder: |map, _| map.stream(),
        }
        .build();
        Py::new(slf.py(), iter)
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

    fn starts_with(&self, str: String) -> MapStartsWithIterator {
        MapStartsWithIteratorBuilder {
            map: self.inner.clone(),
            str,
            stream_builder: |map, str| map.search(Str::new(str).starts_with()).into_stream(),
        }
        .build()
    }

    fn subsequence(&self, str: String) -> MapSubsequenceIterator {
        MapSubsequenceIteratorBuilder {
            map: self.inner.clone(),
            str,
            stream_builder: |map, str| map.search(Subsequence::new(str)).into_stream(),
        }
        .build()
    }

    fn search<'py>(&self, automaton: &AutomatonGraph) -> MapAutomatonIterator {
        MapAutomatonIteratorBuilder {
            map: self.inner.clone(),
            automaton: automaton.get(),
            stream_builder: |map, automaton| map.search(automaton.get()).into_stream(),
        }
        .build()
    }
}

fn fill_map<'py, W: io::Write>(
    iterable: &Bound<'py, PyAny>,
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
pub fn build_map<'py>(iterable: &Bound<'py, PyAny>, path: PathBuf) -> PyResult<Option<Buffer>> {
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
