use super::{native_tails as n, native_return as r};
#[path = "parameter_stream_guard_wrapper_scope.rs"]
mod scope;
pub use scope::{Scope, Services, Resources};
#[path = "parameter_stream_initialization_wrapper_routing.rs"]
mod initialization_routing;
pub use initialization_routing::InitializationServices;
#[path = "parameter_stream_resolver_wrapper_routing.rs"]
mod resolver_routing;
pub use resolver_routing::{ResolverServices, ResolverEntryServices, ResolverProtectedServices, ResolverResolutionServices, ResolverLoadServices, ResolverPriorCleanupServices};
#[path = "parameter_stream_guard_wrapper_finish.rs"]
mod finish;
use finish::finish;
#[derive(Clone, Copy, Debug)]
pub enum Origin { Call(super::CallRequest), Tail(r::TailRequest) }
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub source_pc: u32, pub child: n::ActualEntry }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub origin: Origin, pub wrapper: n::History }
enum Context<T> { Call(super::CallRequest), Tail { request: r::TailRequest, path: r::Path, tail: Option<r::TailCallReturned<T>>,
    captures: r::Captures, phase: r::Phase, attempted: Option<r::Phase>, stopped: bool } }
pub struct ParentState<S: Services> { owners: super::Owners<S>, actual: super::ActualBody, captures: super::Captures,
    phase: super::Phase, attempted: Option<super::Phase>, stopped: bool, context: Context<S::TailOwner>, binding: ActualCall }
pub enum ParentParts<S: Services> { Call { parts: super::Parts<S>, request: Option<super::CallRequest> }, Tail { parts: r::Parts<S>, request: Option<r::TailRequest> } }
impl<S: Services> ParentState<S> {
    fn into_parts(self, services: S) -> ParentParts<S> {
        let parts = super::Parts { services, owners: self.owners, actual: self.actual, captures: self.captures,
            phase: self.phase, attempted: self.attempted, stopped: self.stopped };
        match self.context { Context::Call(request) => ParentParts::Call { parts, request: Some(request) },
            Context::Tail { request, path, tail, captures, phase, attempted, stopped } => ParentParts::Tail {
                parts: r::Parts { body: parts, path, tail, captures, phase, attempted, stopped }, request: Some(request) } }
    }
    fn resources(&mut self) -> Resources<'_, S> {
        let source_tail = match &mut self.context { Context::Call(_) => None, Context::Tail { tail, .. } => Some(tail) };
        Resources { guard: &mut self.owners, source_tail }
    }
    fn origin(&self) -> Origin { match &self.context { Context::Call(r) => Origin::Call(*r), Context::Tail { request, .. } => Origin::Tail(*request) } }
}
pub enum Admission { NotPending, MissingRequest, OccupiedCall, OccupiedTail, CallBinding, ChildEntry }
pub struct Rejected<S: Services> { pub parent: ParentParts<S>, pub actual: ActualCall, pub fault: Admission }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_native_wrapper(mut self, actual: ActualCall) -> Result<n::Prepared<Scope<S>>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        let request = self.captures.request;
        let reject = |parent: Self, fault| Rejected { parent: ParentParts::Call { parts: parent.into_parts(), request }, actual, fault };
        if old || self.phase != super::Phase::Call { return Err(reject(self, Admission::NotPending)); }
        let request = match request { Some(r) => r, None => return Err(reject(self, Admission::MissingRequest)) };
        if self.owners.call.is_some() { return Err(reject(self, Admission::OccupiedCall)); }
        let expected = match request.site { super::CallSite::ZeroConstruct => n::Entry::At57a7e0,
            super::CallSite::IndexedConstruct => n::Entry::At561fa4, super::CallSite::IndexedDestroy => n::Entry::At561fac };
        let indexed_rax = if expected == n::Entry::At57a7e0 { None } else { self.captures.indexed.map(|i| i.base) };
        if actual.source_pc != request.site.pc() || actual.child.entry != expected || actual.child.pc != expected.pc()
            || actual.child.source_image_base != self.actual.source_image_base || actual.child.rsp != request.work_rsp.wrapping_sub(8)
            || actual.child.rcx != request.rcx || (expected != n::Entry::At57a7e0 && indexed_rax != Some(actual.child.rax)) {
            return Err(reject(self, Admission::CallBinding));
        }
        let storage = ParentState { owners: self.owners, actual: self.actual, captures: self.captures, phase: self.phase,
            attempted: self.attempted, stopped: true, context: Context::Call(request), binding: actual };
        begin(Scope { services: self.services }, storage, actual)
    }
}
impl<S: Services> r::Prepared<S> {
    pub fn bind_actual_zero_tail_wrapper(self, actual: ActualCall) -> Result<n::Prepared<Scope<S>>, Rejected<S>> {
        let mut parts = match self.suspend_for_wrapper() { Ok(p) => p, Err(p) => return Err(Rejected {
            parent: ParentParts::Tail { request: p.captures.tail_request, parts: p }, actual, fault: Admission::NotPending }) };
        let rax = match parts.captures.rax { Some(rax) => rax, None => return Err(Rejected {
            parent: ParentParts::Tail { parts, request: None }, actual, fault: Admission::MissingRequest }) };
        let request = r::TailRequest { source_pc: 0x55e0eb, target_rva: 0x57a7fc,
            entry_rsp: parts.body.actual.work_rsp.wrapping_add(0x28), rcx: parts.body.actual.guard_rcx, rax };
        if parts.tail.is_some() { return Err(Rejected { parent: ParentParts::Tail { parts, request: Some(request) }, actual, fault: Admission::OccupiedTail }); }
        if parts.captures.rax != Some(0) || actual.source_pc != request.source_pc || actual.child.entry != n::Entry::At57a7fc
            || actual.child.pc != 0x57a7fc || actual.child.source_image_base != parts.body.actual.source_image_base
            || actual.child.rsp != request.entry_rsp || actual.child.rcx != request.rcx || actual.child.rax != 0 {
            return Err(Rejected { parent: ParentParts::Tail { parts, request: Some(request) }, actual, fault: Admission::CallBinding });
        }
        parts.captures.tail_request = Some(request);
        let storage = ParentState { owners: parts.body.owners, actual: parts.body.actual, captures: parts.body.captures,
            phase: parts.body.phase, attempted: parts.body.attempted, stopped: true, binding: actual,
            context: Context::Tail { request, path: parts.path, tail: parts.tail, captures: parts.captures,
                phase: parts.phase, attempted: parts.attempted, stopped: true } };
        begin(Scope { services: parts.body.services }, storage, actual)
    }
}
fn begin<S: Services>(services: Scope<S>, storage: ParentState<S>, actual: ActualCall) -> Result<n::Prepared<Scope<S>>, Rejected<S>> {
    match n::Prepared::from_actual_entry(n::Input { services, storage, actual: actual.child }) {
        Ok(p) => Ok(p), Err(e) => Err(Rejected { parent: e.input.storage.into_parts(e.input.services.services), actual, fault: Admission::ChildEntry }) }
}
pub struct Retained<C, T> { pub actual: ActualCall, pub origin: Origin, pub call: Option<n::CallReturned<C>>, pub tail: Option<n::TailReturned<T>>,
    pub live_rsp: u64, pub live_rcx: u64, pub live_rax: u64,
    pub history: n::History, pub phase: n::Phase, pub attempted: Option<n::Phase>, pub stopped: bool }
pub struct Preserved<S: Services> { pub parent: ParentParts<S>, pub retained: Retained<S::WrapperCallOwner, S::WrapperTailOwner> }
impl<S: Services> n::Parts<Scope<S>> {
    pub fn into_preserved_guard(self) -> Preserved<S> {
        let state = self.owners.storage;
        let retained = Retained { actual: state.binding, origin: state.origin(), call: self.owners.call, tail: self.owners.tail,
            live_rsp: self.live_rsp, live_rcx: self.live_rcx, live_rax: self.live_rax,
            history: n::History { actual: self.actual, captures: self.captures }, phase: self.phase, attempted: self.attempted, stopped: self.stopped };
        Preserved { parent: state.into_parts(self.services.services), retained }
    }
}
pub enum Supplied<C, T> { Call(super::CallReturned<C>), Tail(r::TailCallReturned<T>) }
pub enum Resumed<S: Services> { Call(super::Prepared<S>), Tail(r::Prepared<S>) }
pub enum ReturnFault<E> { WrongOrigin, ReturnBinding, OccupiedCall, OccupiedTail, Call(super::Fault<E>), Tail(r::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: ParentParts<S>, pub supplied: Option<Supplied<S::CallOwner, S::TailOwner>>,
    pub retained: Option<Retained<S::WrapperCallOwner, S::WrapperTailOwner>>, pub history: History, pub fault: ReturnFault<S::Error> }
impl<S: Services> n::Returned<Scope<S>> {
    pub fn into_guard_parent(self, supplied: Supplied<S::CallOwner, S::TailOwner>) -> Result<Resumed<S>, ReturnRejected<S>> {
        let actual = *self.actual_return(); let parts = self.into_parts(); let state = parts.owners.storage;
        let history = History { actual: state.binding, origin: state.origin(), wrapper: n::History { actual: parts.actual, captures: parts.captures } };
        let mut held = Some(Retained { actual: state.binding, origin: history.origin, call: parts.owners.call, tail: parts.owners.tail,
            live_rsp: parts.live_rsp, live_rcx: parts.live_rcx, live_rax: parts.live_rax,
            history: history.wrapper, phase: parts.phase, attempted: parts.attempted, stopped: parts.stopped });
        let parent = state.into_parts(parts.services.services);
        finish(parent, supplied, actual, history, &mut held).map_err(|(parent, supplied, fault)| ReturnRejected { parent, supplied, retained: held, history, fault })
    }
}
