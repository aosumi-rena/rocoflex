use super::{g, Scope, Services};
use super::super as l;
use g::{native_tails as n, wrapper_call as w};
pub trait WrapperServices: Services {
    type GuardWrapperCallOwner; type GuardWrapperTailOwner;

    fn guard_wrapper_sub(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error>;
    fn guard_wrapper_call(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, request: n::CallRequest) -> Result<(), Self::Error>;
    fn guard_wrapper_lea(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, pc: u32, address: u64, returned: &mut Option<n::LeaReturned>) -> Result<(), Self::Error>;
    fn guard_wrapper_add(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error>;
    fn guard_wrapper_read_target(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn guard_wrapper_tail(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, request: n::TailRequest) -> Result<(), Self::Error>;

    fn adopt_completed_guard_wrapper(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, history: w::History, held: &mut Option<w::Retained<Self::GuardWrapperCallOwner, Self::GuardWrapperTailOwner>>) -> Result<(), Self::Error>;
}
impl<S: WrapperServices> w::Services for Scope<S> {
    type WrapperCallOwner = S::GuardWrapperCallOwner; type WrapperTailOwner = S::GuardWrapperTailOwner;
    fn wrapper_sub(&mut self, r: w::Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.guard_wrapper_sub(&mut r.guard.storage.owners, &mut r.guard.call, r.source_tail, call, tail, returned)
    }
    fn wrapper_call(&mut self, r: w::Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, request: n::CallRequest) -> Result<(), Self::Error> {
        self.services.guard_wrapper_call(&mut r.guard.storage.owners, &mut r.guard.call, r.source_tail, call, tail, request)
    }
    fn wrapper_lea(&mut self, r: w::Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, pc: u32, address: u64, returned: &mut Option<n::LeaReturned>) -> Result<(), Self::Error> {
        self.services.guard_wrapper_lea(&mut r.guard.storage.owners, &mut r.guard.call, r.source_tail, call, tail, pc, address, returned)
    }
    fn wrapper_add(&mut self, r: w::Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.guard_wrapper_add(&mut r.guard.storage.owners, &mut r.guard.call, r.source_tail, call, tail, returned)
    }
    fn wrapper_read_target(&mut self, r: w::Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.guard_wrapper_read_target(&mut r.guard.storage.owners, &mut r.guard.call, r.source_tail, call, tail, pc, address)
    }
    fn wrapper_tail(&mut self, r: w::Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, request: n::TailRequest) -> Result<(), Self::Error> {
        self.services.guard_wrapper_tail(&mut r.guard.storage.owners, &mut r.guard.call, r.source_tail, call, tail, request)
    }
    fn adopt_completed_wrapper(&mut self, r: w::Resources<'_, Self>, history: w::History, held: &mut Option<w::Retained<Self::WrapperCallOwner, Self::WrapperTailOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_guard_wrapper(&mut r.guard.storage.owners, &mut r.guard.call, r.source_tail, history, held)
    }
}

