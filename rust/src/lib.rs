use fst::{Map, MapBuilder, Set, SetBuilder, Streamer};
use pyo3::buffer::PyBuffer;
use pyo3::exceptions::PyBufferError;
use pyo3::types::{PyBytes, PyMemoryView, PyTuple};
use pyo3::{ffi, prelude::*};
use std::ffi::CString;
use std::os::raw::{c_int, c_void};
use std::ptr;
use std::sync::Arc;
use std::{
    fs,
    io::{self, BufWriter},
};

const BUFSIZE: usize = 4 * 1024 * 1024;

#[pyclass]
struct VecBuffer {
    data: Vec<u8>,
}

/// # Safety
///
/// `view` must be a valid pointer to ffi::Py_buffer, or null
/// `data` must outlive the Python lifetime of `owner` (i.e. data must be owned by owner, or data
/// must be static data)
unsafe fn fill_view_from_readonly_data(
    view: *mut ffi::Py_buffer,
    flags: c_int,
    data: &[u8],
    owner: Bound<'_, PyAny>,
) -> PyResult<()> {
    if view.is_null() {
        return Err(PyBufferError::new_err("View is null"));
    }

    if (flags & ffi::PyBUF_WRITABLE) == ffi::PyBUF_WRITABLE {
        return Err(PyBufferError::new_err("Object is not writable"));
    }

    (*view).obj = owner.into_ptr();

    (*view).buf = data.as_ptr() as *mut c_void;
    (*view).len = data.len() as isize;
    (*view).readonly = 1;
    (*view).itemsize = 1;

    (*view).format = if (flags & ffi::PyBUF_FORMAT) == ffi::PyBUF_FORMAT {
        let msg = CString::new("B").unwrap();
        msg.into_raw()
    } else {
        ptr::null_mut()
    };

    (*view).ndim = 1;
    (*view).shape = if (flags & ffi::PyBUF_ND) == ffi::PyBUF_ND {
        &mut (*view).len
    } else {
        ptr::null_mut()
    };

    (*view).strides = if (flags & ffi::PyBUF_STRIDES) == ffi::PyBUF_STRIDES {
        &mut (*view).itemsize
    } else {
        ptr::null_mut()
    };

    (*view).suboffsets = ptr::null_mut();
    (*view).internal = ptr::null_mut();

    Ok(())
}

#[pymethods]
impl VecBuffer {
    unsafe fn __getbuffer__(
        slf: Bound<'_, Self>,
        view: *mut ffi::Py_buffer,
        flags: std::os::raw::c_int,
    ) -> PyResult<()> {
        fill_view_from_readonly_data(view, flags, &slf.borrow().data, slf.into_any())
    }

    unsafe fn __releasebuffer__(&self, view: *mut ffi::Py_buffer) {
        drop(CString::from_raw((*view).format));
    }
}

#[pyclass]
struct PyMap {
    view: PyBuffer<u8>,
    inner: Map<Arc<[u8]>>,
}

fn fill_map<'py, W: io::Write>(
    iterable: &Bound<'py, PyAny>,
    mut builder: MapBuilder<W>,
) -> PyResult<W> {
    let iterator = iterable.iter()?;
    for maybe_obj in iterator {
        let obj = maybe_obj?;
        let tuple = obj.downcast::<PyTuple>()?;
        if tuple.len() != 2 {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Expected 2-tuple (key: bytes, value: int)",
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

/// Build an FST map.
#[pyfunction]
fn build_map<'py>(iterable: &Bound<'py, PyAny>, path: &str) -> PyResult<Option<VecBuffer>> {
    if path == ":memory:" {
        let buf = Vec::with_capacity(10 * (1 << 10));
        let builder = MapBuilder::new(buf)
            .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string()))?;
        let w = fill_map(iterable, builder)?;
        let ret = VecBuffer { data: w };
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
            MapBuilder::new(writer)
                .map_err(|err| PyErr::new::<pyo3::exceptions::PyTypeError, _>(err.to_string()))?,
        )?;
        Ok(None)
    }
}

/// Open an FST map.
#[pyfunction]
fn open_map<'py>(data: &Bound<'py, PyAny>) -> PyResult<PyMap> {
    let view: PyBuffer<u8> = PyBuffer::get_bound(data)?;
    // TODO test view.is_c_contiguous()
    let ptr = view.buf_ptr();
    // TODO arc should probably not try to delete this memory?
    let slice: Arc<[u8]> = unsafe {
        Arc::from_raw(std::slice::from_raw_parts(
            ptr as *const u8,
            view.len_bytes(),
        ))
    };
    let inner = Map::new(slice)
        .map_err(|err| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(err.to_string()))?;
    Ok(PyMap { view, inner })
}

/// A Python module implemented in Rust.
#[pymodule]
fn _fst(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<VecBuffer>()?;
    m.add_function(wrap_pyfunction!(build_map, m)?)?;
    m.add_function(wrap_pyfunction!(open_map, m)?)?;
    Ok(())
}
