use pyo3::prelude::*;

mod buffer;
mod encoding;
mod map;

/// A Python module implemented in Rust.
#[pymodule]
fn _fst(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<buffer::Buffer>()?;
    m.add_function(wrap_pyfunction!(encoding::encode_int, m)?)?;
    m.add_function(wrap_pyfunction!(encoding::decode_int, m)?)?;
    m.add_function(wrap_pyfunction!(map::map_from_iterable, m)?)?;
    m.add_function(wrap_pyfunction!(map::map, m)?)?;
    Ok(())
}
