use std::collections::TryReserveError;
use crate::{conditional_gate::{Batch, BorrowedProjection, GateError, Projection},
    logical_read_rows::ReadRows, model::ModelNode, model_tags::decoded_hashes,
    modm0_logical_sequence::{self as sequence, CachePolicy, LogicalSequence, RequiredReset}};

const MODULE: u32 = 0x7fca41ff;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind { Moda5, Moda7, Modl1, Modl0 }
impl Kind {
    fn name(self) -> &'static str {
        match self { Self::Moda5 => "moda5", Self::Moda7 => "moda7", Self::Modl1 => "modl1", Self::Modl0 => "modl0" }
    }
}
pub enum Child<I, A, C, L> { Moda5(I), Moda7(A), Modl1(C), Modl0(L) }
impl<I, A, C, L> Child<I, A, C, L> {
    pub fn kind(&self) -> Kind {
        match self { Self::Moda5(_) => Kind::Moda5, Self::Moda7(_) => Kind::Moda7,
            Self::Modl1(_) => Kind::Modl1, Self::Modl0(_) => Kind::Modl0 }
    }
}
impl<I: Projection, A: Projection, C: Projection, L: Projection> Projection for Child<I, A, C, L> {
    fn reset(&mut self) -> Result<(), GateError> {
        match self { Self::Moda5(child) => child.reset(), Self::Moda7(child) => child.reset(),
            Self::Modl1(child) => child.reset(), Self::Modl0(child) => child.reset() }
    }
    fn forward(&mut self, input: &Batch) -> Result<Batch, GateError> {
        match self { Self::Moda5(child) => child.forward(input), Self::Moda7(child) => child.forward(input),
            Self::Modl1(child) => child.forward(input), Self::Modl0(child) => child.forward(input) }
    }
}
impl<I: BorrowedProjection, A: BorrowedProjection, C: BorrowedProjection, L: BorrowedProjection>
    BorrowedProjection for Child<I, A, C, L>
{
    fn forward_borrowed(&mut self, input: &dyn ReadRows) -> Result<Batch, GateError> {
        match self { Self::Moda5(child) => child.forward_borrowed(input), Self::Moda7(child) => child.forward_borrowed(input),
            Self::Modl1(child) => child.forward_borrowed(input), Self::Modl0(child) => child.forward_borrowed(input) }
    }
}
#[derive(Debug)]
pub enum DecodeError {
    Sequence(String), Configuration(usize), Empty, CountDomain,
    UnknownChild { index: usize, kind: u32, tag: u64 },
    LeafShape { index: usize, operation: Kind, bytes: usize, children: usize },
    Allocation(TryReserveError),
}
pub struct Node<'n, 'data> { kind: Kind, node: &'n ModelNode<'data> }
impl<'n, 'data> Node<'n, 'data> {
    pub fn kind(&self) -> Kind { self.kind }
    pub fn node(&self) -> &'n ModelNode<'data> { self.node }
}
pub struct Decoded<'n, 'data> { node: &'n ModelNode<'data>, children: Vec<Node<'n, 'data>> }
impl<'n, 'data> Decoded<'n, 'data> {
    pub fn node(&self) -> &'n ModelNode<'data> { self.node }
    pub fn children(&self) -> &[Node<'n, 'data>] { &self.children }

    pub fn bind_initialized<I: BorrowedProjection, A: BorrowedProjection,
        C: BorrowedProjection, L: BorrowedProjection, R>(self, mut parts: Parts<I, A, C, L, R>)
        -> Result<Bound<'n, 'data, I, A, C, L, R>, BindingFailure<'n, 'data, I, A, C, L, R>>
        where R: RequiredReset<Child<I, A, C, L>, Child<I, A, C, L>>
    {
        if parts.children.len() != self.children.len() {
            return Err(BindingFailure::Metadata { reason: Mismatch::Count {
                expected: self.children.len(), actual: parts.children.len() }, parameters: self, parts });
        }
        if let Some((index, expected, actual)) = self.children.iter().zip(&parts.children).enumerate()
            .find_map(|(index, (expected, actual))| (expected.kind != actual.kind())
                .then_some((index, expected.kind, actual.kind())))
        {
            return Err(BindingFailure::Metadata { reason: Mismatch::Kind { index, expected, actual }, parameters: self, parts });
        }

        let first = parts.children.remove(0);
        let Parts { children: following, intermediates, reset, cache_policy, needs_reset } = parts;
        match LogicalSequence::from_explicit_parts(sequence::Parts {
            first, following, intermediates, reset, cache_policy, needs_reset,
        }) {
            Ok(projection) => Ok(Bound { parameters: self, projection }),
            Err(refusal) => Err(BindingFailure::State { parameters: self, refusal }),
        }
    }
}
pub struct Parts<I, A, C, L, R> {
    pub children: Vec<Child<I, A, C, L>>, pub intermediates: Vec<Batch>,
    pub reset: R, pub cache_policy: CachePolicy, pub needs_reset: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mismatch { Count { expected: usize, actual: usize }, Kind { index: usize, expected: Kind, actual: Kind } }
pub struct Bound<'n, 'data, I, A, C, L, R>
    where R: RequiredReset<Child<I, A, C, L>, Child<I, A, C, L>>
{
    pub parameters: Decoded<'n, 'data>,
    pub projection: LogicalSequence<Child<I, A, C, L>, Child<I, A, C, L>, R>,
}
pub enum BindingFailure<'n, 'data, I, A, C, L, R> {
    Metadata { reason: Mismatch, parameters: Decoded<'n, 'data>, parts: Parts<I, A, C, L, R> },
    State { parameters: Decoded<'n, 'data>, refusal: sequence::Refusal<Child<I, A, C, L>, Child<I, A, C, L>, R> },
}

pub fn decode_node<'n, 'data>(node: &'n ModelNode<'data>) -> Result<Decoded<'n, 'data>, DecodeError> {
    node.require(MODULE, b"modm0", decoded_hashes("modm0")).map_err(DecodeError::Sequence)?;
    if !node.data.is_empty() { return Err(DecodeError::Configuration(node.data.len())); }
    if node.children.is_empty() { return Err(DecodeError::Empty); }
    if node.children.len() > i32::MAX as usize { return Err(DecodeError::CountDomain); }
    let mut children = Vec::new();
    children.try_reserve_exact(node.children.len()).map_err(DecodeError::Allocation)?;
    for (index, child) in node.children.iter().enumerate() {
        let kind = [Kind::Moda5, Kind::Moda7, Kind::Modl1, Kind::Modl0].into_iter()
            .find(|kind| child.require(MODULE, kind.name().as_bytes(), decoded_hashes(kind.name())).is_ok())
            .ok_or(DecodeError::UnknownChild { index, kind: child.kind, tag: child.tag })?;
        if matches!(kind, Kind::Moda5 | Kind::Moda7) && (!child.data.is_empty() || !child.children.is_empty()) {
            return Err(DecodeError::LeafShape { index, operation: kind, bytes: child.data.len(), children: child.children.len() });
        }
        children.push(Node { kind, node: child });
    }
    Ok(Decoded { node, children })
}
