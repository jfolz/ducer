use fst::{map::Stream, Streamer};
use ouroboros::self_referencing;
use pyo3::{
    buffer::PyBuffer,
    prelude::*,
    types::{PyBytes, PyTuple},
};
use std::{
    fs,
    io::{self, BufWriter},
    sync::Arc,
};

mod buffer;
use buffer::{Buffer, PyBufferRef};

#[pyfunction]
fn encode_int<'py>(py: Python<'py>, i: u64) -> PyResult<Bound<'py, PyBytes>> {
    // +3 for bits needed to encode length
    let num_bits = u64::BITS - (i | 1).leading_zeros() + 3;
    let num_bytes = (num_bits + 7) >> 3;
    if num_bytes > 8 {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "int {i} > 2305843009213693951"
        )));
    }
    let enc = (i << 3) | ((num_bytes - 1) as u64);
    let enc = enc.to_be_bytes();
    let (_, enc) = enc.split_at((u64::BITS / 8 - num_bytes) as usize);
    Ok(PyBytes::new_bound(py, enc))
}

#[pyfunction]
fn decode_int<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyTuple>> {
    let num_bytes = ((*data
        .last()
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyValueError, _>("empty"))?
        & 0b111)
        + 1) as usize;
    if num_bytes > data.len() {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "incomplete int",
        ));
    }
    let mut i = 0u64;
    let n = data.len() - num_bytes;
    let (rem, data) = data.split_at(n);
    for &b in data {
        i = (i << 8) | u64::from(b);
    }
    // 3 bits were used to encode the length
    i >>= 3;

    let rem = PyBytes::new_bound(py, rem).into_py(py);
    Ok(PyTuple::new_bound(py, [rem, i.into_py(py)]))
}

const BUFSIZE: usize = 4 * 1024 * 1024;

#[pyclass]
struct Map {
    inner: Arc<fst::Map<PyBufferRef>>,
}

#[pymethods]
impl Map {
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<MapIterator>> {
        let iter = MapIterator {
            map: slf.inner.clone(),
            stream_builder: |map| map.stream(),
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
}

#[pyclass]
#[self_referencing]
struct MapIterator {
    map: Arc<fst::Map<PyBufferRef>>,
    #[borrows(map)]
    #[not_covariant]
    stream: Stream<'this>,
}

#[pymethods]
impl MapIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyObject> {
        let py = slf.py();
        match slf.with_stream_mut(|stream| stream.next()) {
            Some((key, val)) => {
                let k = PyBytes::new_bound(py, key).into_py(py);
                let v = val.to_object(py);
                let t = PyTuple::new_bound(py, [k, v]);
                Some(t.into_py(py))
            }
            None => None,
        }
    }
}

fn fill_map<'py, W: io::Write>(
    iterable: &Bound<'py, PyAny>,
    mut builder: fst::MapBuilder<W>,
) -> PyResult<W> {
    let iterator = iterable.iter()?;
    for maybe_obj in iterator {
        let obj = maybe_obj?;
        let tuple = obj.downcast::<PyTuple>()?;
        if tuple.len() != 2 {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Expected tuple (key: bytes, value: int)",
            ));
        }
        let item0 = tuple.get_item(0)?;
        let key = item0.downcast::<PyBytes>()?.as_bytes();
        let val = tuple.get_item(1)?.extract::<u64>()?;
        builder
            .insert(key, val)
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string()))?;
    }
    builder
        .into_inner()
        .map_err(|err| PyErr::new::<pyo3::exceptions::PyIOError, _>(err.to_string()))
}

/// Build an FST map from an iterable for tuples (key: bytes, value: int).
#[pyfunction]
fn map_from_iterable<'py>(iterable: &Bound<'py, PyAny>, path: &str) -> PyResult<Option<Buffer>> {
    if path == ":memory:" {
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

/// Open an FST map.
#[pyfunction]
fn map<'py>(data: &Bound<'py, PyAny>) -> PyResult<Map> {
    let view: PyBuffer<u8> = PyBuffer::get_bound(data)?;
    let slice = PyBufferRef::new(view);
    let inner = Arc::new(
        fst::Map::new(slice)
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string()))?,
    );
    Ok(Map { inner })
}

/// A Python module implemented in Rust.
#[pymodule]
fn _fst(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Buffer>()?;
    m.add_function(wrap_pyfunction!(encode_int, m)?)?;
    m.add_function(wrap_pyfunction!(decode_int, m)?)?;
    m.add_function(wrap_pyfunction!(map_from_iterable, m)?)?;
    m.add_function(wrap_pyfunction!(map, m)?)?;
    Ok(())
}
