use pyo3::{
    buffer::{Element, PyBuffer},
    exceptions::PyBufferError,
    ffi,
    prelude::*,
};
use std::{
    ffi::CString,
    os::raw::{c_int, c_void},
    ptr,
};

#[pyclass]
pub struct Buffer {
    data: Vec<u8>,
}

impl Buffer {
    pub fn new(data: Vec<u8>) -> Self {
        Buffer { data }
    }
}

#[pymethods]
impl Buffer {
    unsafe fn __getbuffer__(
        slf: Bound<'_, Self>,
        view: *mut ffi::Py_buffer,
        flags: c_int,
    ) -> PyResult<()> {
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

pub struct PyBufferRef<T: Element> {
    view: PyBuffer<T>,
}

impl<T: Element> PyBufferRef<T> {
    pub fn new(view: PyBuffer<T>) -> Self {
        Self { view }
    }
}

impl<T: Element> AsRef<[T]> for PyBufferRef<T> {
    fn as_ref(&self) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(self.view.buf_ptr() as *const T, self.view.len_bytes())
        }
    }
}
