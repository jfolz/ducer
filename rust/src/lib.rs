use pyo3::prelude::*;

mod automaton;
mod buffer;
mod map;
mod set;

/// A Python module implemented in Rust.
#[pymodule]
fn _fst(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<buffer::Buffer>()?;
    m.add_class::<map::Map>()?;
    m.add_class::<map::Op>()?;
    m.add_class::<set::Set>()?;
    m.add_class::<automaton::AutomatonGraph>()?;
    Ok(())
}
