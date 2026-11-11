use super::{formatting as f, sentry, facet, finish, ret, Scope, Services, ActualCall};
#[derive(Clone, Copy, Debug)]
pub struct NativeProgress { pub history: ret::History, pub phase: ret::Phase, pub pop: usize,
    pub attempted: Option<(ret::Phase, usize)>, pub stopped: bool }
pub struct Stage<R, C, P> { pub calls: R, pub captures: C, pub phase: P, pub attempted: Option<P>, pub stopped: bool }
pub struct BodyRetained<C> { pub constructor: Option<f::ConstructorReturned<C>>, pub actual: f::ActualBody,
    pub captures: f::Captures, pub saved: Option<f::Saved>, pub phase: f::Phase, pub attempted: Option<f::Phase>, pub stopped: bool }
pub struct Retained<C, A, B, D> { pub actual: ActualCall, pub request: super::super::CallRequest, pub body: BodyRetained<C>,
    pub sentry: Option<Stage<sentry::Calls<A>, sentry::Captures, sentry::Phase>>,
    pub facet: Option<Stage<facet::Calls<B>, facet::Captures, facet::Phase>>,
    pub finish: Option<Stage<finish::Calls<D>, finish::Captures, finish::Phase>>, pub native: Option<NativeProgress> }
pub struct Preserved<S: Services> { pub parent: super::super::Parts<S>,
    pub retained: Retained<S::FormatConstructorOwner, S::FormatSentryOwner, S::FormatFacetOwner, S::FormatFinishOwner> }
fn preserve_body<S: Services>(body: f::Parts<Scope<S>>) -> Preserved<S> {
    let state = body.owners.storage;
    let retained = Retained { actual: state.binding, request: state.request,
        body: BodyRetained { constructor: body.owners.constructor, actual: body.actual, captures: body.captures,
            saved: body.saved, phase: body.phase, attempted: body.attempted, stopped: body.stopped },
        sentry: None, facet: None, finish: None, native: None };
    Preserved { parent: state.into_parent(body.services.services).into_parts(), retained }
}
fn preserve_sentry<S: Services>(parts: sentry::Parts<Scope<S>>) -> Preserved<S> {
    let mut p = preserve_body(parts.prefix);
    p.retained.sentry = Some(Stage { calls: parts.calls, captures: parts.captures, phase: parts.phase, attempted: parts.attempted, stopped: parts.stopped }); p
}
fn preserve_facet<S: Services>(parts: facet::Parts<Scope<S>>) -> Preserved<S> {
    let mut p = preserve_sentry(parts.sentry);
    p.retained.facet = Some(Stage { calls: parts.calls, captures: parts.captures, phase: parts.phase, attempted: parts.attempted, stopped: parts.stopped }); p
}
fn preserve_finish<S: Services>(parts: finish::Parts<Scope<S>>) -> Preserved<S> {
    let mut p = match parts.prior { finish::Prior::Sentry(s) => preserve_sentry(s), finish::Prior::Facet(f) => preserve_facet(f) };
    p.retained.finish = Some(Stage { calls: parts.calls, captures: parts.captures, phase: parts.phase, attempted: parts.attempted, stopped: parts.stopped }); p
}
pub(super) fn preserve_native<S: Services>(parts: ret::Parts<Scope<S>>) -> Preserved<S> {
    let mut p = preserve_finish(parts.body);
    p.retained.native = Some(NativeProgress { history: ret::History { input: parts.input, required_rax: parts.required_rax, captures: parts.captures },
        phase: parts.phase, pop: parts.pop, attempted: parts.attempted, stopped: parts.stopped }); p
}
impl<S: Services> f::Parts<Scope<S>> { pub fn into_preserved_number(self) -> Preserved<S> { preserve_body(self) } }
impl<S: Services> sentry::Parts<Scope<S>> { pub fn into_preserved_number(self) -> Preserved<S> { preserve_sentry(self) } }
impl<S: Services> facet::Parts<Scope<S>> { pub fn into_preserved_number(self) -> Preserved<S> { preserve_facet(self) } }
impl<S: Services> finish::Parts<Scope<S>> { pub fn into_preserved_number(self) -> Preserved<S> { preserve_finish(self) } }
impl<S: Services> ret::Parts<Scope<S>> { pub fn into_preserved_number(self) -> Preserved<S> { preserve_native(self) } }
