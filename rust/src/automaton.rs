use fst::automaton::Automaton;
use pyo3::{prelude::*, types::PyType};
use std::sync::Arc;

#[inline]
fn nevermatch_start() -> State {
    State::NeverMatch
}

#[inline]
fn nevermatch_is_match() -> bool {
    false
}

#[inline]
fn nevermatch_can_match() -> bool {
    false
}

#[inline]
fn nevermatch_will_always_match() -> bool {
    false
}

#[inline]
fn nevermatch_accept() -> State {
    State::NeverMatch
}

#[inline]
fn alwaysmatch_start() -> State {
    State::AlwaysMatch
}

#[inline]
fn alwaysmatch_is_match() -> bool {
    true
}

#[inline]
fn alwaysmatch_can_match() -> bool {
    true
}

#[inline]
fn alwaysmatch_will_always_match() -> bool {
    true
}

#[inline]
fn alwaysmatch_accept() -> State {
    State::AlwaysMatch
}

#[inline]
fn str_start() -> State {
    State::Str(Some(0))
}

#[inline]
fn str_is_match(str: &[u8], pos: &Option<usize>) -> bool {
    *pos == Some(str.len())
}

#[inline]
fn str_can_match(pos: &Option<usize>) -> bool {
    pos.is_some()
}

#[inline]
fn str_accept(str: &[u8], pos: &Option<usize>, byte: u8) -> State {
    // if we aren't already past the end...
    if let Some(pos) = *pos {
        // and there is still a matching byte at the current position...
        if str.get(pos).copied() == Some(byte) {
            // then move forward
            return State::Str(Some(pos + 1));
        }
    }
    // otherwise we're either past the end or didn't match the byte
    State::Str(None)
}

#[inline]
fn subsequence_start() -> State {
    State::Subsequence(0)
}

#[inline]
fn subsequence_is_match(node: &[u8], state: usize) -> bool {
    state == node.len()
}

#[inline]
fn subsequence_can_match() -> bool {
    true
}

#[inline]
fn subsequence_will_always_match(node: &[u8], state: usize) -> bool {
    state == node.len()
}

#[inline]
fn subsequence_accept(node: &[u8], state: usize, byte: u8) -> State {
    if state == node.len() {
        return State::Subsequence(state);
    }
    State::Subsequence(state + usize::from(byte == node[state]))
}

#[derive(Debug)]
pub enum StartsWithState {
    Done,
    Running(State),
}

#[inline]
fn starts_with_start(node: &Node) -> State {
    State::StartsWith(Box::new({
        let inner = node.start();
        if node.is_match(&inner) {
            StartsWithState::Done
        } else {
            StartsWithState::Running(inner)
        }
    }))
}

#[inline]
fn starts_with_is_match(state: &StartsWithState) -> bool {
    match state {
        StartsWithState::Done => true,
        StartsWithState::Running(_) => false,
    }
}

#[inline]
fn starts_with_can_match(node: &Node, state: &StartsWithState) -> bool {
    match state {
        StartsWithState::Done => true,
        StartsWithState::Running(ref inner) => node.can_match(inner),
    }
}

#[inline]
fn starts_with_will_always_match(state: &StartsWithState) -> bool {
    match state {
        StartsWithState::Done => true,
        StartsWithState::Running(_) => false,
    }
}

#[inline]
fn starts_with_accept(node: &Node, state: &StartsWithState, byte: u8) -> State {
    State::StartsWith(Box::new(match state {
        StartsWithState::Done => StartsWithState::Done,
        StartsWithState::Running(ref inner) => {
            let next_inner = node.accept(inner, byte);
            if node.is_match(&next_inner) {
                StartsWithState::Done
            } else {
                StartsWithState::Running(next_inner)
            }
        }
    }))
}

#[derive(Debug)]
pub struct UnionState(State, State);

#[inline]
fn union_start(node: &(Arc<Node>, Arc<Node>)) -> State {
    State::Union(Box::new(UnionState(node.0.start(), node.1.start())))
}

#[inline]
fn union_is_match(node: &(Arc<Node>, Arc<Node>), state: &UnionState) -> bool {
    node.0.is_match(&state.0) || node.1.is_match(&state.1)
}

#[inline]
fn union_can_match(node: &(Arc<Node>, Arc<Node>), state: &UnionState) -> bool {
    node.0.can_match(&state.0) || node.1.can_match(&state.1)
}

#[inline]
fn union_will_always_match(node: &(Arc<Node>, Arc<Node>), state: &UnionState) -> bool {
    node.0.will_always_match(&state.0) || node.1.will_always_match(&state.1)
}

#[inline]
fn union_accept(node: &(Arc<Node>, Arc<Node>), state: &UnionState, byte: u8) -> State {
    State::Union(Box::new(UnionState(
        node.0.accept(&state.0, byte),
        node.1.accept(&state.1, byte),
    )))
}

#[derive(Debug)]
pub struct IntersectionState(State, State);

#[inline]
fn intersection_start(node: &(Arc<Node>, Arc<Node>)) -> State {
    let s = Box::new(IntersectionState(node.0.start(), node.1.start()));
    State::Intersection(s)
}

#[inline]
fn intersection_is_match(node: &(Arc<Node>, Arc<Node>), state: &IntersectionState) -> bool {
    node.0.is_match(&state.0) && node.1.is_match(&state.1)
}

#[inline]
fn intersection_can_match(node: &(Arc<Node>, Arc<Node>), state: &IntersectionState) -> bool {
    node.0.can_match(&state.0) && node.1.can_match(&state.1)
}

#[inline]
fn intersection_will_always_match(
    node: &(Arc<Node>, Arc<Node>),
    state: &IntersectionState,
) -> bool {
    node.0.will_always_match(&state.0) && node.1.will_always_match(&state.1)
}

#[inline]
fn intersection_accept(
    node: &(Arc<Node>, Arc<Node>),
    state: &IntersectionState,
    byte: u8,
) -> State {
    State::Intersection(Box::new(IntersectionState(
        node.0.accept(&state.0, byte),
        node.1.accept(&state.1, byte),
    )))
}

#[derive(Debug)]
pub struct ComplementState(State);

#[inline]
fn complement_start(node: &Node) -> State {
    State::Complement(Box::new(ComplementState(node.start())))
}

#[inline]
fn complement_is_match(node: &Node, state: &ComplementState) -> bool {
    !node.is_match(&state.0)
}

#[inline]
fn complement_can_match(node: &Node, state: &ComplementState) -> bool {
    !node.will_always_match(&state.0)
}

#[inline]
fn complement_will_always_match(node: &Node, state: &ComplementState) -> bool {
    !node.can_match(&state.0)
}

#[inline]
fn complement_accept(node: &Node, state: &ComplementState, byte: u8) -> State {
    State::Complement(Box::new(ComplementState(node.accept(&state.0, byte))))
}

#[derive(Debug)]
pub enum State {
    NeverMatch,
    AlwaysMatch,
    Str(Option<usize>),
    Subsequence(usize),
    StartsWith(Box<StartsWithState>),
    Complement(Box<ComplementState>),
    Intersection(Box<IntersectionState>),
    Union(Box<UnionState>),
}

#[derive(Debug)]
pub enum Node {
    NeverMatch,
    AlwaysMatch,
    Str(Vec<u8>),
    Subsequence(Vec<u8>),
    StartsWith(Arc<Node>),
    Complement(Arc<Node>),
    Intersection((Arc<Node>, Arc<Node>)),
    Union((Arc<Node>, Arc<Node>)),
}

impl Automaton for Node {
    type State = State;

    fn start(&self) -> State {
        match self {
            Self::NeverMatch => nevermatch_start(),
            Self::AlwaysMatch => alwaysmatch_start(),
            Self::Str(_) => str_start(),
            Self::Subsequence(_) => subsequence_start(),
            Self::StartsWith(n) => starts_with_start(n),
            Self::Complement(n) => complement_start(n),
            Self::Intersection(n) => intersection_start(n),
            Self::Union(n) => union_start(n),
        }
    }

    fn is_match(&self, state: &State) -> bool {
        match (self, state) {
            (Self::NeverMatch, State::NeverMatch) => nevermatch_is_match(),
            (Self::AlwaysMatch, State::AlwaysMatch) => alwaysmatch_is_match(),
            (Self::Str(n), State::Str(state)) => str_is_match(n, state),
            (Self::Subsequence(n), State::Subsequence(state)) => subsequence_is_match(n, *state),
            (Self::StartsWith(_), State::StartsWith(state)) => starts_with_is_match(state),
            (Self::Complement(n), State::Complement(state)) => complement_is_match(n, state),
            (Self::Intersection(n), State::Intersection(state)) => intersection_is_match(n, state),
            (Self::Union(n), State::Union(state)) => union_is_match(n, state),
            _ => panic!("type mismatch: node {self:?} state {state:?}"),
        }
    }

    fn can_match(&self, state: &Self::State) -> bool {
        // true
        match (self, state) {
            (Self::NeverMatch, State::NeverMatch) => nevermatch_can_match(),
            (Self::AlwaysMatch, State::AlwaysMatch) => alwaysmatch_can_match(),
            (Self::Str(_), State::Str(state)) => str_can_match(state),
            (Self::Subsequence(_), State::Subsequence(_)) => subsequence_can_match(),
            (Self::StartsWith(n), State::StartsWith(state)) => starts_with_can_match(n, state),
            (Self::Complement(n), State::Complement(state)) => complement_can_match(n, state),
            (Self::Intersection(n), State::Intersection(state)) => intersection_can_match(n, state),
            (Self::Union(n), State::Union(state)) => union_can_match(n, state),
            _ => panic!("type mismatch: node {self:?} state {state:?}"),
        }
    }

    fn will_always_match(&self, state: &Self::State) -> bool {
        // false
        match (self, state) {
            (Self::NeverMatch, State::NeverMatch) => nevermatch_will_always_match(),
            (Self::AlwaysMatch, State::AlwaysMatch) => alwaysmatch_will_always_match(),
            (Self::Str(_), State::Str(_)) => false,
            (Self::Subsequence(n), State::Subsequence(state)) => {
                subsequence_will_always_match(n, *state)
            }
            (Self::StartsWith(_), State::StartsWith(state)) => starts_with_will_always_match(state),
            (Self::Complement(n), State::Complement(state)) => {
                complement_will_always_match(n, state)
            }
            (Self::Intersection(n), State::Intersection(state)) => {
                intersection_will_always_match(n, state)
            }
            (Self::Union(n), State::Union(state)) => union_will_always_match(n, state),
            _ => panic!("type mismatch: node {self:?} state {state:?}"),
        }
    }

    fn accept(&self, state: &State, byte: u8) -> State {
        match (self, state) {
            (Self::NeverMatch, State::NeverMatch) => nevermatch_accept(),
            (Self::AlwaysMatch, State::AlwaysMatch) => alwaysmatch_accept(),
            (Self::Str(n), State::Str(state)) => str_accept(n, state, byte),
            (Self::Subsequence(n), State::Subsequence(state)) => {
                subsequence_accept(n, *state, byte)
            }
            (Self::StartsWith(n), State::StartsWith(state)) => starts_with_accept(n, state, byte),
            (Self::Complement(n), State::Complement(state)) => complement_accept(n, state, byte),
            (Self::Intersection(n), State::Intersection(state)) => {
                intersection_accept(n, state, byte)
            }
            (Self::Union(n), State::Union(state)) => union_accept(n, state, byte),
            _ => panic!("type mismatch: node {self:?} state {state:?}"),
        }
    }

    fn accept_eof(&self, _: &Self::State) -> Option<Self::State> {
        None
    }
}

pub struct ArcNode(Arc<Node>);

impl ArcNode {
    pub fn get(&self) -> ArcNode {
        ArcNode(self.0.clone())
    }
}

impl Automaton for ArcNode {
    type State = State;

    fn start(&self) -> State {
        self.0.as_ref().start()
    }

    fn is_match(&self, state: &State) -> bool {
        self.0.as_ref().is_match(state)
    }

    fn can_match(&self, state: &Self::State) -> bool {
        self.0.as_ref().can_match(state)
    }

    fn will_always_match(&self, state: &Self::State) -> bool {
        self.0.as_ref().will_always_match(state)
    }

    fn accept(&self, state: &State, byte: u8) -> State {
        self.0.as_ref().accept(state, byte)
    }

    fn accept_eof(&self, state: &Self::State) -> Option<Self::State> {
        self.0.as_ref().accept_eof(state)
    }
}

/// Automata can be used to efficiently apply complex search patterns
/// to the keys of maps and sets.
/// Use one of the classmethods never, always, str,
/// or subsequence to create a new automaton.
/// Add more complex behavior on top with starts_with, complement,
/// intersection, or union.
/// E.g., an automaton that matches keys that start with b"foo" or b"bar":
///
///     a_foo = Automaton.str(b"foo")
///     a_bar = Automaton.str(b"bar")
///     a_foobar = a_foo.union(a_bar).starts_with()
#[pyclass(name = "Automaton", subclass)]
#[allow(clippy::module_name_repetitions)]
pub struct AutomatonGraph {
    root: Arc<Node>,
}

impl AutomatonGraph {
    pub fn get(&self) -> ArcNode {
        ArcNode(self.root.clone())
    }
}

#[pymethods]
impl AutomatonGraph {
    /// Create a new Automaton that never matches.
    #[classmethod]
    fn never(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            root: Arc::new(Node::NeverMatch),
        }
    }

    /// Create a new Automaton that always matches.
    #[classmethod]
    fn always(_cls: &Bound<'_, PyType>) -> Self {
        Self {
            root: Arc::new(Node::AlwaysMatch),
        }
    }

    /// Create a new Automaton that matches str exactly.
    #[classmethod]
    fn str(_cls: &Bound<'_, PyType>, str: &[u8]) -> Self {
        Self {
            root: Arc::new(Node::Str(str.to_owned())),
        }
    }

    /// Create a new Automaton that subsequences matches str.
    /// E.g., b"bd" matches the key b"abcde".
    #[classmethod]
    fn subsequence(_cls: &Bound<'_, PyType>, str: &[u8]) -> Self {
        Self {
            root: Arc::new(Node::Subsequence(str.to_owned())),
        }
    }

    /// Modify this automaton to match any key that starts with a prefix that previously matched,
    /// e.g., if self matched b"abc", it will now match b"abcde".
    /// Returns self to allow chaining with other methods.
    fn starts_with(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.root = Arc::new(Node::StartsWith(slf.root.clone()));
        slf
    }

    /// Modify this automaton to match any key that would previously not match.
    /// Returns self to allow chaining with other methods.
    fn complement(mut slf: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        slf.root = Arc::new(Node::Complement(slf.root.clone()));
        slf
    }

    /// Modify this automaton to match any key that both self and other matches.
    /// other must be an instance of Automaton.
    /// Returns self to allow chaining with other methods.
    fn intersection<'py>(
        mut slf: PyRefMut<'py, Self>,
        other: &AutomatonGraph,
    ) -> PyRefMut<'py, Self> {
        slf.root = Arc::new(Node::Intersection((slf.root.clone(), other.root.clone())));
        slf
    }

    /// Modify this automaton to match any key that either self or other matches.
    /// other must be an instance of Automaton.
    /// Returns self to allow chaining with other methods.
    fn union<'py>(mut slf: PyRefMut<'py, Self>, other: &AutomatonGraph) -> PyRefMut<'py, Self> {
        slf.root = Arc::new(Node::Union((slf.root.clone(), other.root.clone())));
        slf
    }
}
