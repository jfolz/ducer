use pyo3::{
    prelude::*,
    types::{PyBytes, PyTuple},
};

#[pyfunction]
pub fn encode_int<'py>(py: Python<'py>, i: u64) -> PyResult<Bound<'py, PyBytes>> {
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
pub fn decode_int<'py>(py: Python<'py>, data: &[u8]) -> PyResult<Bound<'py, PyTuple>> {
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
