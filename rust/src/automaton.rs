use fst::{
    automaton::{
        self, AlwaysMatch, Automaton as _, Complement, Intersection, StartsWith, Str, Subsequence,
        Union,
    },
    Map, Set,
};
use ouroboros::self_referencing;
use pyo3::prelude::*;

#[pyclass]
#[self_referencing]
pub struct Automaton {
    str: Box<String>,
    #[borrows(str)]
    #[not_covariant]
    inner: Box<dyn Send + Sync + 'this>,
}

#[pyclass]
pub struct AlwaysMatchAutomata {
    inner: AlwaysMatch,
}

#[pyclass]
#[self_referencing]
pub struct StrAutomata {
    str: Box<String>,
    #[borrows(str)]
    #[not_covariant]
    inner: Str<'this>,
}

impl StrAutomata {
    fn from(str: &str) -> Self {
        StrAutomataBuilder {
            str: Box::new(str.to_owned()),
            inner_builder: |parent| Str::new(&parent),
        }
        .build()
    }

    pub fn automata<'a>(&'a self) -> &Str<'a> {
        self.with_inner(|str| str)
    }
}

#[pymethods]
impl StrAutomata {
    pub fn starts_with(&self) -> StartsWithAutomata {
        StartsWithAutomata::from(self.with_str(|str| Self::from(str)))
    }
}

#[pyclass]
#[self_referencing]
pub struct StartsWithAutomata {
    parent: StrAutomata,
    #[borrows(parent)]
    #[not_covariant]
    inner: StartsWith<Str<'this>>,
}

impl StartsWithAutomata {
    fn from(parent: StrAutomata) -> Self {
        StartsWithAutomataBuilder {
            parent,
            inner_builder: |parent| parent.automata().clone().starts_with(),
        }
        .build()
    }

    pub fn automata<'a>(&'a self) -> &StartsWith<Str<'a>> {
        self.with_inner(|str| str)
    }
}

#[pyfunction]
pub fn starts_with<'py>(start: &str) -> StartsWithAutomata {
    StrAutomata::from(start).starts_with()
}

#[pyfunction]
pub fn always<'py>() -> AlwaysMatchAutomata {
    AlwaysMatchAutomata { inner: AlwaysMatch }
}
