use super::*;
use crate::{conditional_gate::Batch, paired_adopt as adopt};
#[path = "paired_call_stages.rs"]
mod stages;
#[path = "paired_call_finish.rs"]
mod finish;
#[path = "paired_call_second_append.rs"]
mod second_append_step;
#[path = "paired_call_filter.rs"]
mod filter_step;
pub use filter_step::{SecondOwners, FilterOwners, FilteredParts, JoinedFilterParts};

pub struct Continuation { halted: bool, completed_calls: u64 }
impl Continuation {
    pub fn from_explicit_parts(halted: bool, completed_calls: u64) -> Self { Self { halted, completed_calls } }
    pub fn is_halted(&self) -> bool { self.halted }
    pub fn completed_calls(&self) -> u64 { self.completed_calls }
    pub fn into_parts(self) -> (bool, u64) { (self.halted, self.completed_calls) }
}
pub struct Parts<M, R, W, A, B, O, S, U: ScalarUpper, P, G> {
    pub continuation: Continuation, pub scalar: input::State<M, R>,
    pub condition: condition::State<W>, pub engine: join::Engine<A, B>, pub packing: packing::State<O>,
    pub interpolation: interpolation::State<S, U>, pub features: features::State<P, G>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Domain { PriorFailure, Groups, Frame, Rate, CursorRoles, Ended, CursorHalted, CompletedCount }
pub struct BindingFailure<M, R, W, A, B, O, S, U: ScalarUpper, P, G> {
    pub cause: Domain, pub parts: Parts<M, R, W, A, B, O, S, U, P, G>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Binding {
    pub groups: usize, pub hop: usize, pub factor: i32, pub ended: u8,
    pub worker_cursor: i32, pub post_cursor: i32, pub post_enabled: bool, pub logical_cursor_count: usize,
}
pub struct Bound<M, R, W, A, B, O, S, U: ScalarUpper, P, G> {
    parts: Parts<M, R, W, A, B, O, S, U, P, G>, binding: Binding, next_completed_calls: u64,
}
impl<M, R, W, A, B, O, S, U: ScalarUpper, P, G> Bound<M, R, W, A, B, O, S, U, P, G> {
    pub fn from_explicit_parts(parts: Parts<M, R, W, A, B, O, S, U, P, G>)
        -> Result<Self, BindingFailure<M, R, W, A, B, O, S, U, P, G>> {
        let scalar = parts.scalar.parts(); let condition = parts.condition.parts();
        let post = parts.engine.parts(); let packing = parts.packing.parts();
        let interpolation = parts.interpolation.parts(); let features = parts.features.parts();

        let cause = if parts.continuation.halted || parts.scalar.is_halted() || parts.condition.is_halted()
            || parts.packing.is_halted() || parts.interpolation.is_halted() || parts.features.is_halted() {
            Some(Domain::PriorFailure)
        } else if scalar.groups == 0 || scalar.groups != post.groups || scalar.groups != features.groups {
            Some(Domain::Groups)
        } else if scalar.hop != packing.frame { Some(Domain::Frame) }
        else if scalar.hop % scalar.groups != 0 || interpolation.factor24 <= 0
            || scalar.hop / scalar.groups != interpolation.factor24 as usize { Some(Domain::Rate) }
        else if condition.cursor_index508 != 0 || (post.enabled && post.cursor_index512 != 1)
            || condition.cursor_count != if post.enabled { 2 } else { 1 } { Some(Domain::CursorRoles) }
        else if scalar.ended517 != condition.ended517 || scalar.ended517 != post.ended
            || scalar.ended517 != packing.ended { Some(Domain::Ended) }
        else if condition.cursor_halted { Some(Domain::CursorHalted) }
        else { None };
        if let Some(cause) = cause { return Err(BindingFailure { cause, parts }); }
        let Some(next_completed_calls) = parts.continuation.completed_calls.checked_add(1) else {
            return Err(BindingFailure { cause: Domain::CompletedCount, parts });
        };
        let binding = Binding { groups: scalar.groups, hop: scalar.hop, factor: interpolation.factor24,
            ended: scalar.ended517, worker_cursor: condition.cursor_index508, post_cursor: post.cursor_index512,
            post_enabled: post.enabled, logical_cursor_count: condition.cursor_count };
        Ok(Self { parts, binding, next_completed_calls })
    }
    pub fn parts(&self) -> &Parts<M, R, W, A, B, O, S, U, P, G> { &self.parts }
    pub fn binding(&self) -> Binding { self.binding }
    pub fn begin<H>(self, signal: Batch, paired_features: Batch, held: H)
        -> Result<Carried<input::Prepared<M, R, History<H>>, ScalarOwners<W, A, B, O, S, U, P, G>>,
            ScalarStartFailure<M, R, H, W, A, B, O, S, U, P, G>> where M: post::Module, R: input::Rows {
        let Parts { mut continuation, scalar, condition, engine, packing, interpolation, features } = self.parts;

        continuation.halted = true;
        let history = History { held, continuation, binding: self.binding, next_completed_calls: self.next_completed_calls };
        let owners = ScalarOwners { condition, engine, packing, next: NextOwners { interpolation, features } };
        match scalar.begin(signal, paired_features, history) {
            Ok(child) => Ok(Carried { child, held: owners }),
            Err(start) => Err(ScalarStartFailure { start, owners }),
        }
    }
}

pub struct History<H> { held: H, continuation: Continuation, binding: Binding, next_completed_calls: u64 }
impl<H> History<H> {
    pub fn held(&self) -> &H { &self.held }
    pub fn continuation(&self) -> &Continuation { &self.continuation }
    pub fn binding(&self) -> Binding { self.binding }

    fn finish(self) -> (H, ValueCallComplete) {
        let Self { held, mut continuation, binding, next_completed_calls } = self;
        continuation.halted = false; continuation.completed_calls = next_completed_calls;
        (held, ValueCallComplete { continuation, binding })
    }
}
pub struct NextOwners<S, U: ScalarUpper, P, G> { interpolation: interpolation::State<S, U>, features: features::State<P, G> }
impl<S, U: ScalarUpper, P, G> NextOwners<S, U, P, G> {
    pub fn interpolation(&self) -> &interpolation::State<S, U> { &self.interpolation }
    pub fn features(&self) -> &features::State<P, G> { &self.features }
}
pub struct ScalarOwners<W, A, B, O, S, U: ScalarUpper, P, G> {
    condition: condition::State<W>, engine: join::Engine<A, B>, packing: packing::State<O>, next: NextOwners<S, U, P, G>,
}
impl<W, A, B, O, S, U: ScalarUpper, P, G> ScalarOwners<W, A, B, O, S, U, P, G> {
    pub fn condition(&self) -> &condition::State<W> { &self.condition }
    pub fn engine(&self) -> &join::Engine<A, B> { &self.engine }
    pub fn packing(&self) -> &packing::State<O> { &self.packing }
    pub fn next(&self) -> &NextOwners<S, U, P, G> { &self.next }
}

pub struct ScalarStartFailure<M, R, H, W, A, B, O, S, U: ScalarUpper, P, G> {
    start: input::StartFailure<M, R, History<H>>, owners: ScalarOwners<W, A, B, O, S, U, P, G>,
}
impl<M, R, H, W, A, B, O, S, U: ScalarUpper, P, G> ScalarStartFailure<M, R, H, W, A, B, O, S, U, P, G> {
    pub fn start(&self) -> &input::StartFailure<M, R, History<H>> { &self.start }
    pub fn owners(&self) -> &ScalarOwners<W, A, B, O, S, U, P, G> { &self.owners }
}

pub struct ValueCallComplete { continuation: Continuation, binding: Binding }
impl ValueCallComplete {
    pub fn continuation(&self) -> &Continuation { &self.continuation }
    pub fn binding(&self) -> Binding { self.binding }
    pub fn into_parts(self) -> (Continuation, Binding) { (self.continuation, self.binding) }
}

pub struct CompletedCall { adopted: Adopted, complete: ValueCallComplete }
impl CompletedCall {
    pub fn adoption(&self) -> &Adopted { &self.adopted }
    pub fn completion(&self) -> &ValueCallComplete { &self.complete }
    pub fn into_parts(self) -> (Adopted, ValueCallComplete) { (self.adopted, self.complete) }
}
