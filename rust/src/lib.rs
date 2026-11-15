use pyo3::prelude::*;

mod automaton;
mod buffer;
mod map;
mod set;

// fst performs only limited verification of inputs and otherwise assumes
// well-formed data.
// Corrupt or truncated data can make fst panic.
// Guard every fst entry point so a Rust panic surfaces to Python as
// RuntimeError instead of PanicException.
pub(crate) fn fst_guard<T>(op: impl FnOnce() -> T) -> PyResult<T> {
    fst_guard_py(move || Ok(op()))
}

pub(crate) fn fst_guard_py<T>(op: impl FnOnce() -> PyResult<T>) -> PyResult<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(op)) {
        Ok(result) => result,
        Err(payload) => {
            let detail = payload
                .downcast_ref::<&'static str>()
                .map(|s| (*s).to_string())
                .or_else(|| payload.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".to_string());
            Err(pyo3::exceptions::PyRuntimeError::new_err(format!(
                "fst operation failed: {detail}"
            )))
        }
    }
}

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
