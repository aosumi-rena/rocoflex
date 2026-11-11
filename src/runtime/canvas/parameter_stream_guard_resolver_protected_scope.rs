use super::{r, i, Scope, Request, ActualCall};
use r::protected_call as p;
pub trait Services: super::Services {
    type ResolverProtectedOwner;
    fn resolver_protected_setup(&mut self, o: &mut i::Owners<Self>, request: p::InstructionRequest,
        returned: &mut Option<p::InstructionReturned>) -> Result<(), Self::Error>;

    fn resolver_protected_call(&mut self, o: &mut i::Owners<Self>, request: p::CallRequest,
        returned: &mut Option<p::Exit<Self::ResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
pub trait ResolutionServices: Services {
    fn retain_resolver_resolution(&mut self, o: &mut i::Owners<Self>, history: p::resolution::History,
        held: &mut Option<p::Exit<Self::ResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
pub trait LoadServices: Services {
    fn retain_resolver_load_exit(&mut self, o: &mut i::Owners<Self>, history: p::load_publication::History,
        held: &mut Option<p::Exit<Self::ResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
impl<S: Services> p::Services for Scope<S> {
    type ProtectedOwner = S::ResolverProtectedOwner;
    fn setup_instruction(&mut self, o: &mut Self::Storage, request: p::InstructionRequest,
        returned: &mut Option<p::InstructionReturned>) -> Result<(), Self::Error> {
        self.services.resolver_protected_setup(&mut o.owners, request, returned)
    }
    fn protected_call(&mut self, o: &mut Self::Storage, request: p::CallRequest,
        returned: &mut Option<p::Exit<Self::ProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.resolver_protected_call(&mut o.owners, request, returned)
    }
}
impl<S: ResolutionServices> p::resolution::Services for Scope<S> {
    fn retain_resolution_exit(&mut self, o: &mut Self::Storage, history: p::resolution::History,
        held: &mut Option<p::Exit<Self::ProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.retain_resolver_resolution(&mut o.owners, history, held)
    }
}
impl<S: LoadServices> p::load_publication::next_call::Services for Scope<S> {
    fn retain_load_exit(&mut self, o: &mut Self::Storage, history: p::load_publication::History,
        held: &mut Option<p::Exit<Self::ProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.retain_resolver_load_exit(&mut o.owners, history, held)
    }
}
pub struct Retained<R> { pub binding: ActualCall, pub request: Request, pub body: r::Captures,
    pub actual: p::ActualBoundary, pub requirement: r::ProtectedRequirement, pub captures: p::Captures,
    pub registers: p::Registers, pub rsp: u64, pub index: usize, pub phase: p::Phase,
    pub attempted: Option<(p::Phase, usize)>, pub stopped: bool, pub exit: Option<p::Exit<R>> }
pub struct Preserved<S: Services> { pub parent: i::Parts<S>, pub retained: Retained<S::ResolverProtectedOwner> }
impl<S: Services> p::Parts<Scope<S>> {
    pub fn into_preserved_initializer(self) -> Preserved<S> {
        let state = self.prefix.storage;
        let retained = Retained { binding: state.binding, request: state.request, body: self.prefix.captures,
            actual: self.actual, requirement: self.requirement, captures: self.captures, registers: self.registers,
            rsp: self.rsp, index: self.index, phase: self.phase, attempted: self.attempted, stopped: self.stopped, exit: self.exit };
        Preserved { parent: state.into_parent(self.prefix.services.services).into_parts(), retained }
    }
}
pub struct ResolutionPreserved<S: ResolutionServices> { pub protected: Preserved<S>, pub actual: p::resolution::ActualContinuation,
    pub captures: p::resolution::Captures, pub phase: p::resolution::Phase, pub attempted: Option<p::resolution::Phase>,
    pub history: Option<p::resolution::History>, pub stopped: bool }
impl<S: ResolutionServices> p::resolution::Parts<Scope<S>> {
    pub fn into_preserved_initializer(self) -> ResolutionPreserved<S> {
        ResolutionPreserved { protected: self.protected.into_preserved_initializer(), actual: self.actual, captures: self.captures,
            phase: self.phase, attempted: self.attempted, history: self.history, stopped: self.stopped }
    }
}
pub struct LoadPreserved<S: Services> { pub protected: Preserved<S>, pub actual: p::load_publication::ActualContinuation,
    pub captures: p::load_publication::Captures, pub phase: p::load_publication::Phase,
    pub attempted: Option<p::load_publication::Phase>, pub stopped: bool }
impl<S: Services> p::load_publication::Parts<Scope<S>> {
    pub fn into_preserved_initializer(self) -> LoadPreserved<S> {
        LoadPreserved { protected: self.protected.into_preserved_initializer(), actual: self.actual,
            captures: self.captures, phase: self.phase, attempted: self.attempted, stopped: self.stopped }
    }
}

pub trait PriorCleanupServices: Services {
    fn retain_resolver_prior_cleanup_exit(&mut self, o: &mut i::Owners<Self>, history: p::prior_cleanup::History,
        held: &mut Option<p::Exit<Self::ResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
impl<S: PriorCleanupServices> p::prior_cleanup::Services for Scope<S> {
    fn retain_prior_cleanup_exit(&mut self, o: &mut Self::Storage, history: p::prior_cleanup::History,
        held: &mut Option<p::Exit<Self::ProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.retain_resolver_prior_cleanup_exit(&mut o.owners, history, held)
    }
}
