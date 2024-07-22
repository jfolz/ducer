use pyo3::prelude::*;

mod automaton;
mod buffer;
mod encoding;
mod map;
mod set;

/// A Python module implemented in Rust.
#[pymodule]
fn _fst(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<buffer::Buffer>()?;
    m.add_function(wrap_pyfunction!(encoding::encode_int, m)?)?;
    m.add_function(wrap_pyfunction!(encoding::decode_int, m)?)?;
    m.add_class::<map::Map>()?;
    m.add_class::<set::Set>()?;
    m.add_function(wrap_pyfunction!(set::build_from_iterable, m)?)?;
    m.add_class::<automaton::AutomatonGraph>()?;
    Ok(())
}
