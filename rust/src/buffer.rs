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

/// A read-only buffer returned by Map.build and Set.build
/// when path is ":memory:".
/// Use to create new Map or Set instances, or write to file:
///
///     from ducer import Set
///     buf = Set.build([b"a", b"b"], ":memory:")
///     s = Set(buf)
///     for k in s:
///         print(k)
///     with open("my.set", "wb") as f:
///         f.write(buf)
#[pyclass(subclass)]
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
    #[allow(clippy::cast_possible_wrap)]
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
        // check if casting data.len() to isize has wrapped around
        if (*view).len < 0 {
            return Err(PyBufferError::new_err("Buffer is too large"));
        }
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

    #[allow(clippy::unused_self)]
    unsafe fn __releasebuffer__(&self, view: *mut ffi::Py_buffer) {
        // drop the format string, if any
        let fmt = (*view).format;
        if !fmt.is_null() {
            drop(CString::from_raw(fmt));
        }
    }

    fn __len__(&self) -> usize {
        self.data.len()
    }
}

/// Holds a `PyBuffer<T>` and creates a `&[T]` slice from it.
pub struct PyBufferRef<T: Element> {
    view: PyBuffer<T>,
}

impl<T: Element> PyBufferRef<T> {
    /// Create a new `PyBufferRef` from the given `PyBuffer`.
    /// Returns `PyValueError` if buffer is not contiguous.
    pub fn new(view: PyBuffer<T>) -> PyResult<Self> {
        if view.is_c_contiguous() || view.is_fortran_contiguous() {
            Ok(Self { view })
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "buffer must be contiguous",
            ))
        }
    }
}

impl<T: Element> AsRef<[T]> for PyBufferRef<T> {
    fn as_ref(&self) -> &[T] {
        let ptr = self.view.buf_ptr() as *const T;
        // Check that the pointer is properly aligned
        assert!(
            (ptr as usize) % std::mem::align_of::<T>() == 0,
            "PyBuffer pointer is not properly aligned"
        );
        unsafe {
            // Safety:
            // We have to assume that the `PyBuffer` is implemented correctly, so
            // - `ptr` is a valid pointer
            // - we just checked that it's aligned properly for `T`
            // - we already checked in `new` that it's contiguous
            // - `item_count()` correctly returns the number of items
            std::slice::from_raw_parts(ptr, self.view.item_count())
        }
    }
}
