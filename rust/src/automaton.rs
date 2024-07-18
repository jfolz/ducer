/*
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


enum AutomatonState {
    AlwaysMatch,
    Str(Option<usize>),
    Subsequence(usize),
    StartsWith(StartsWithState<GraphAutomatonNode>),
    Complement(ComplementState<GraphAutomatonNode>),
    Intersection(IntersectionState<GraphAutomatonNode, GraphAutomatonNode>),
    Union(UnionState<GraphAutomatonNode, GraphAutomatonNode>),
}

#[derive(Clone)]
enum GraphAutomatonRoot {
    Str(String),
    Subsequence(String),
}

impl Automaton for GraphAutomatonRoot {
    type State = AutomatonState;

    fn start(&self) -> Self::State {
        match self {
            Self::Str(str) => Self::State::Str(Str::new(&str).start()),
            Self::Subsequence(str) => Self::State::Subsequence(Subsequence::new(&str).start()),
        }
    }

    fn is_match(&self, state: &Self::State) -> bool {
        match (self, state) {
            (Self::Str(str), Self::State::Str(state)) => Str::new(&str).is_match(state),
            (Self::Subsequence(str), Self::State::Subsequence(state)) => {
                Subsequence::new(&str).is_match(state)
            }
            _ => panic!(
                "type mismatch: node {:?} state {:?}",
                self.type_id(),
                state.type_id()
            ),
        }
    }

    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        match (self, state) {
            (Self::Str(str), Self::State::Str(state)) => {
                Self::State::Str(Str::new(&str).accept(state, byte))
            }
            (Self::Subsequence(str), Self::State::Subsequence(state)) => {
                Self::State::Subsequence(Subsequence::new(&str).accept(state, byte))
            }
            _ => panic!(
                "type mismatch: node {:?} state {:?}",
                self.type_id(),
                state.type_id()
            ),
        }
    }
}

#[derive(Clone)]
enum GraphAutomatonNode {
    Root(GraphAutomatonRoot),
    AlwaysMatch,
    StartsWith(Box<GraphAutomatonNode>),
    Complement(Box<GraphAutomatonNode>),
    Intersection(Box<(GraphAutomatonNode, GraphAutomatonNode)>),
    Union(Box<(GraphAutomatonNode, GraphAutomatonNode)>),
}

impl Automaton for GraphAutomatonNode {
    type State = AutomatonState;

    fn start(&self) -> Self::State {
        match self {
            Self::Root(a) => a.start(),
            Self::AlwaysMatch => Self::State::AlwaysMatch,
            Self::StartsWith(a) => Self::State::StartsWith(a.clone().starts_with().start()),
            Self::Complement(a) => Self::State::Complement(a.clone().complement().start()),
            Self::Intersection(v) => {
                let v = v.clone();
                Self::State::Intersection(v.0.intersection(v.1).start())
            }
            Self::Union(v) => {
                let v = v.clone();
                Self::State::Union(v.0.union(v.1).start())
            }
        }
    }

    fn is_match(&self, state: &Self::State) -> bool {
        match (self, state) {
            (Self::Root(a), state) => a.is_match(state),
            (Self::AlwaysMatch, Self::State::AlwaysMatch) => true,
            (Self::StartsWith(a), Self::State::StartsWith(state)) => {
                a.clone().starts_with().is_match(state)
            }
            (Self::Complement(a), Self::State::Complement(state)) => {
                a.clone().complement().is_match(state)
            }
            (Self::Intersection(v), Self::State::Intersection(state)) => {
                let v = v.clone();
                v.0.intersection(v.1).is_match(state)
            }
            (Self::Union(v), Self::State::Union(state)) => {
                let v = v.clone();
                v.0.union(v.1).is_match(state)
            }
            _ => panic!(
                "type mismatch: node {:?} state {:?}",
                self.type_id(),
                state.type_id()
            ),
        }
    }

    fn accept(&self, state: &Self::State, byte: u8) -> Self::State {
        match (self, state) {
            (Self::Root(a), state) => a.accept(state, byte),
            (Self::AlwaysMatch, Self::State::AlwaysMatch) => Self::State::AlwaysMatch,
            (Self::StartsWith(a), Self::State::StartsWith(state)) => {
                Self::State::StartsWith(a.starts_with().accept(state, byte))
            }
            (Self::Complement(a), Self::State::Complement(state)) => {
                Self::State::Complement(a.complement().accept(state, byte))
            }
            (Self::Intersection(v), Self::State::Intersection(state)) => {
                Self::State::Intersection(v.0.intersection(v.1).accept(state, byte))
            }
            (Self::Union(v), Self::State::Union(state)) => {
                Self::State::Union(v.0.union(v.1).accept(state, byte))
            }
            _ => panic!(
                "type mismatch: node {:?} state {:?}",
                self.type_id(),
                state.type_id()
            ),
        }
    }
}


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
