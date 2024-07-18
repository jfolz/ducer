use std::{any::Any, vec};

use fst::{
    automaton::{
        self, AlwaysMatch, Automaton, Complement, ComplementState, Intersection, IntersectionState,
        StartsWith, StartsWithState, Str, Subsequence, Union, UnionState,
    },
    map, set, IntoStreamer, Map, Set,
};
use ouroboros::self_referencing;
use pyo3::prelude::*;

mod fst_fork;

/*
fn apply_automaton<'a, T: AsRef<[u8]>>(
    instructions: AutomatonType,
    map: &'a Map<T>,
) -> map::Stream<'a> {
    let mut a: Box<dyn Any>;
    AlwaysMatch.accept_eof(&());
    loop {
        match instructions {
            AutomatonType::AlwaysMatch => {
                a = Box::new(AlwaysMatch);
                break;
            }
            AutomatonType::Str(str) => {
                a = Box::new(Str::new(&str));
                break;
            }
            AutomatonType::Subsequence(str) => {
                a = Box::new(Subsequence::new(&str));
                break;
            }
            AutomatonType::StartsWith(other) => todo!(),
            AutomatonType::Complement(other) => todo!(),
            AutomatonType::Intersection(first, second) => todo!(),
            AutomatonType::Union(first, second) => todo!(),
        }
    }
    map.search(*a).into_stream()
}

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
*/
