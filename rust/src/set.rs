use fst::{set::Stream, Streamer};
use ouroboros::self_referencing;
use pyo3::{buffer::PyBuffer, prelude::*, types::PyBytes};
use std::{
    fs,
    io::{self, BufWriter},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::buffer::{Buffer, PyBufferRef};

const BUFSIZE: usize = 4 * 1024 * 1024;

#[pyclass(sequence)]
pub struct Set {
    inner: Arc<fst::Set<PyBufferRef<u8>>>,
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
        let inner =
            Arc::new(fst::Set::new(slice).map_err(|err| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string())
            })?);
        Ok(Self { inner })
    }

    #[allow(clippy::needless_pass_by_value)]
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<SetIterator>> {
        let iter = SetIteratorBuilder {
            set: slf.inner.clone(),
            stream_builder: |set| set.stream(),
        }
        .build();
        Py::new(slf.py(), iter)
    }

    fn __contains__(&self, key: &[u8]) -> bool {
        self.inner.contains(key)
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }
}

#[pyclass]
#[self_referencing]
struct SetIterator {
    set: Arc<fst::Set<PyBufferRef<u8>>>,
    #[borrows(set)]
    #[not_covariant]
    stream: Stream<'this>,
}

#[pymethods]
impl SetIterator {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>) -> Option<PyObject> {
        let py = slf.py();
        match slf.with_stream_mut(|stream| stream.next()) {
            Some(key) => {
                let k = PyBytes::new_bound(py, key).into_py(py);
                Some(k)
            }
            None => None,
        }
    }
}

fn fill<W: io::Write>(iterable: &Bound<'_, PyAny>, mut builder: fst::SetBuilder<W>) -> PyResult<W> {
    let iterator = iterable.iter()?;
    for maybe_obj in iterator {
        let obj = maybe_obj?;
        let key = obj.extract::<&[u8]>()?;
        builder
            .insert(key)
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyValueError, _>(err.to_string()))?;
    }
    builder
        .into_inner()
        .map_err(|err| PyErr::new::<pyo3::exceptions::PyIOError, _>(err.to_string()))
}

/// Build a Set from an iterable of `bytes`
/// and write it to the given path.
/// If path is `:memory:`, returns a `Buffer` containing the set data.
#[pyfunction(name = "build_set")]
pub fn build(iterable: &Bound<'_, PyAny>, path: PathBuf) -> PyResult<Option<Buffer>> {
    if path == Path::new(":memory:") {
        let buf = Vec::with_capacity(10 * (1 << 10));
        let builder = fst::SetBuilder::new(buf)
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string()))?;
        let w = fill(iterable, builder)?;
        let ret = Buffer::new(w);
        Ok(Some(ret))
    } else {
        let wp = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        let writer = BufWriter::with_capacity(BUFSIZE, wp);
        fill(
            iterable,
            fst::SetBuilder::new(writer)
                .map_err(|err| PyErr::new::<pyo3::exceptions::PyTypeError, _>(err.to_string()))?,
        )?;
        Ok(None)
    }
}
