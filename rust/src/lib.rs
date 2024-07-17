use fst::{map::Stream, Streamer};
use ouroboros::self_referencing;
use pyo3::{
    buffer::PyBuffer,
    exceptions::PyBufferError,
    ffi,
    prelude::*,
    types::{PyBytes, PyTuple},
};
use std::{
    ffi::CString,
    fs,
    io::{self, BufWriter},
    os::raw::{c_int, c_void},
    ptr,
    sync::Arc,
};

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
struct Buffer {
    data: Vec<u8>,
}

#[pymethods]
impl Buffer {
    unsafe fn __getbuffer__(
        slf: Bound<'_, Self>,
        view: *mut ffi::Py_buffer,
        flags: std::os::raw::c_int,
    ) -> PyResult<()> {
        //fill_view_from_readonly_data(view, flags, &slf.borrow().data, slf.into_any())
        if view.is_null() {
            return Err(PyBufferError::new_err("Py_buffer must not be null"));
        }
        if (flags & ffi::PyBUF_WRITABLE) == ffi::PyBUF_WRITABLE {
            return Err(PyBufferError::new_err("Buffer is read-only"));
        }

        let data = &slf.borrow().data;
        // Pointer to the wrapped Vec
        // This is safe, since slf owns the underlying Vec
        (*view).buf = data.as_ptr() as *mut c_void;
        (*view).len = data.len() as isize;
        (*view).readonly = 1;
        (*view).itemsize = 1;
        (*view).ndim = 1;

        // Python C-API:
        // If set, this field MUST be filled in correctly.
        // Otherwise, this field MUST be NULL.
        (*view).format = if (flags & ffi::PyBUF_FORMAT) == ffi::PyBUF_FORMAT {
            let msg = CString::new("B").unwrap();
            msg.into_raw()
        } else {
            ptr::null_mut()
        };

        // Set 1D shape if requested
        (*view).shape = if (flags & ffi::PyBUF_ND) == ffi::PyBUF_ND {
            &mut (*view).len
        } else {
            ptr::null_mut()
        };

        // Set stride 1 if requested
        (*view).strides = if (flags & ffi::PyBUF_STRIDES) == ffi::PyBUF_STRIDES {
            &mut (*view).itemsize
        } else {
            ptr::null_mut()
        };

        (*view).suboffsets = ptr::null_mut();
        (*view).internal = ptr::null_mut();

        // bind to self to ensure slf lives long enough
        (*view).obj = slf.into_ptr();
        Ok(())
    }

    unsafe fn __releasebuffer__(&self, view: *mut ffi::Py_buffer) {
        // drop the format string, if any
        let fmt = (*view).format;
        if !fmt.is_null() {
            drop(CString::from_raw(fmt));
        }
    }
}

struct UnsafeRef {
    ptr: *const u8,
    len: usize,
}

unsafe impl Send for UnsafeRef {}

unsafe impl Sync for UnsafeRef {}

impl AsRef<[u8]> for UnsafeRef {
    fn as_ref(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

#[pyclass]
struct Map {
    view: PyBuffer<u8>,
    inner: Arc<fst::Map<UnsafeRef>>,
}

#[pymethods]
impl Map {
    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<Py<FstMapIterator>> {
        let iter = FstMapIteratorBuilder {
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
struct FstMapIterator {
    map: Arc<fst::Map<UnsafeRef>>,
    #[borrows(map)]
    #[not_covariant]
    stream: Stream<'this>,
}

#[pymethods]
impl FstMapIterator {
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
        let ret = Buffer { data: w };
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
    // TODO test view.is_c_contiguous()
    let slice = UnsafeRef {
        ptr: view.buf_ptr() as *const u8,
        len: view.len_bytes(),
    };
    let inner = Arc::new(
        fst::Map::new(slice)
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string()))?,
    );
    Ok(Map { view, inner })
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
