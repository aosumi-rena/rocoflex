use super::{l, Scope, GuardServices};
use super::super as c;
use l::{guard as g, guard_call as gc};
use g::{native_tails as n, wrapper_call as w};
pub trait WrapperServices: GuardServices {
    type LookupGuardWrapperCallOwner; type LookupGuardWrapperTailOwner;

    fn lookup_guard_wrapper_sub(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, guard: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::LookupGuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::LookupGuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::LookupGuardWrapperTailOwner>>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error>;
    fn lookup_guard_wrapper_call(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, guard: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::LookupGuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::LookupGuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::LookupGuardWrapperTailOwner>>, request: n::CallRequest) -> Result<(), Self::Error>;
    fn lookup_guard_wrapper_lea(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, guard: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::LookupGuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::LookupGuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::LookupGuardWrapperTailOwner>>, pc: u32, address: u64, returned: &mut Option<n::LeaReturned>) -> Result<(), Self::Error>;
    fn lookup_guard_wrapper_add(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, guard: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::LookupGuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::LookupGuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::LookupGuardWrapperTailOwner>>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error>;
    fn lookup_guard_wrapper_read_target(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, guard: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::LookupGuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::LookupGuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::LookupGuardWrapperTailOwner>>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn lookup_guard_wrapper_tail(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, guard: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::LookupGuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::LookupGuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::LookupGuardWrapperTailOwner>>, request: n::TailRequest) -> Result<(), Self::Error>;

    fn adopt_completed_lookup_guard_wrapper(&mut self, o: &mut c::Owners<Self>, lookup: &mut l::Calls<Self::LookupCallOwner>, guard: &mut Option<g::CallReturned<Self::LookupGuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::LookupGuardTailOwner>>>, history: w::History, held: &mut Option<w::Retained<Self::LookupGuardWrapperCallOwner, Self::LookupGuardWrapperTailOwner>>) -> Result<(), Self::Error>;
}
impl<S: WrapperServices> gc::WrapperServices for Scope<S> {
    type GuardWrapperCallOwner = S::LookupGuardWrapperCallOwner; type GuardWrapperTailOwner = S::LookupGuardWrapperTailOwner;
    fn guard_wrapper_sub(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.lookup_guard_wrapper_sub(&mut o.storage.owners, &mut o.calls, guard, source_tail, call, tail, returned)
    }
    fn guard_wrapper_call(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, request: n::CallRequest) -> Result<(), Self::Error> {
        self.services.lookup_guard_wrapper_call(&mut o.storage.owners, &mut o.calls, guard, source_tail, call, tail, request)
    }
    fn guard_wrapper_lea(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, pc: u32, address: u64, returned: &mut Option<n::LeaReturned>) -> Result<(), Self::Error> {
        self.services.lookup_guard_wrapper_lea(&mut o.storage.owners, &mut o.calls, guard, source_tail, call, tail, pc, address, returned)
    }
    fn guard_wrapper_add(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.lookup_guard_wrapper_add(&mut o.storage.owners, &mut o.calls, guard, source_tail, call, tail, returned)
    }
    fn guard_wrapper_read_target(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.lookup_guard_wrapper_read_target(&mut o.storage.owners, &mut o.calls, guard, source_tail, call, tail, pc, address)
    }
    fn guard_wrapper_tail(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, call: &mut Option<n::CallReturned<Self::GuardWrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::GuardWrapperTailOwner>>, request: n::TailRequest) -> Result<(), Self::Error> {
        self.services.lookup_guard_wrapper_tail(&mut o.storage.owners, &mut o.calls, guard, source_tail, call, tail, request)
    }
    fn adopt_completed_guard_wrapper(&mut self, o: &mut l::Owners<Self>, guard: &mut Option<g::CallReturned<Self::GuardCallOwner>>, source_tail: Option<&mut Option<g::native_return::TailCallReturned<Self::GuardTailOwner>>>, history: w::History, held: &mut Option<w::Retained<Self::GuardWrapperCallOwner, Self::GuardWrapperTailOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_lookup_guard_wrapper(&mut o.storage.owners, &mut o.calls, guard, source_tail, history, held)
    }
}

