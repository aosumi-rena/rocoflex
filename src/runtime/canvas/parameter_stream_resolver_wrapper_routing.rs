use super::{n, Scope, Resources, InitializationServices};
use n::{initialization as i, initialization_call as ic};
use i::resolver_prefix::{self as r, selected_call as rc};
pub trait ResolverServices: InitializationServices {

    fn wrapper_resolver_read_word(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn wrapper_resolver_read_dword(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn wrapper_resolver_xchg(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, pc: u32, address: u64, word: u64, returned: &mut Option<r::ExchangeReturned>) -> Result<(), Self::Error>;
    fn wrapper_resolver_restore(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, pc: u32, register: r::native_return::Register, address: u64, returned: &mut Option<r::native_return::Restored>) -> Result<(), Self::Error>;
    fn wrapper_resolver_add(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, returned: &mut Option<r::StackAdjustment>) -> Result<(), Self::Error>;
    fn wrapper_resolver_pop(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, pc: u32, register: r::native_return::Register, returned: &mut Option<r::Pop>) -> Result<(), Self::Error>;
    fn wrapper_resolver_ret(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, returned: &mut Option<r::native_return::ActualReturn>) -> Result<(), Self::Error>;
    fn wrapper_resolver_adopt(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, history: rc::History, held: &mut Option<rc::Retained>) -> Result<(), Self::Error>;
}
pub trait ResolverEntryServices: ResolverServices {
    fn wrapper_resolver_entry(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, request: r::native_entry::Request, returned: &mut Option<r::native_entry::InstructionReturned>) -> Result<(), Self::Error>;
}
impl<S: ResolverServices> ic::ResolverServices for Scope<S> {
    fn initialization_resolver_read_word(&mut self, o: &mut ic::ResolverResources<'_, Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.wrapper_resolver_read_word(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, pc, address)
    }
    fn initialization_resolver_read_dword(&mut self, o: &mut ic::ResolverResources<'_, Self>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.wrapper_resolver_read_dword(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, pc, address)
    }
    fn initialization_resolver_xchg(&mut self, o: &mut ic::ResolverResources<'_, Self>, pc: u32, address: u64, word: u64, returned: &mut Option<r::ExchangeReturned>) -> Result<(), Self::Error> {
        self.services.wrapper_resolver_xchg(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, pc, address, word, returned)
    }
    fn initialization_resolver_restore(&mut self, o: &mut ic::ResolverResources<'_, Self>, pc: u32, register: r::native_return::Register, address: u64, returned: &mut Option<r::native_return::Restored>) -> Result<(), Self::Error> {
        self.services.wrapper_resolver_restore(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, pc, register, address, returned)
    }
    fn initialization_resolver_add(&mut self, o: &mut ic::ResolverResources<'_, Self>, returned: &mut Option<r::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.wrapper_resolver_add(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, returned)
    }
    fn initialization_resolver_pop(&mut self, o: &mut ic::ResolverResources<'_, Self>, pc: u32, register: r::native_return::Register, returned: &mut Option<r::Pop>) -> Result<(), Self::Error> {
        self.services.wrapper_resolver_pop(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, pc, register, returned)
    }
    fn initialization_resolver_ret(&mut self, o: &mut ic::ResolverResources<'_, Self>, returned: &mut Option<r::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.wrapper_resolver_ret(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, returned)
    }
    fn adopt_initialization_resolver(&mut self, o: &mut ic::ResolverResources<'_, Self>, history: rc::History, held: &mut Option<rc::Retained>) -> Result<(), Self::Error> {
        self.services.wrapper_resolver_adopt(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, history, held)
    }
}
impl<S: ResolverEntryServices> ic::ResolverEntryServices for Scope<S> {
    fn initialization_resolver_entry(&mut self, o: &mut ic::ResolverResources<'_, Self>, request: r::native_entry::Request, returned: &mut Option<r::native_entry::InstructionReturned>) -> Result<(), Self::Error> {
        self.services.wrapper_resolver_entry(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, request, returned)
    }
}

pub trait ResolverProtectedServices: ResolverServices {
    type WrapperResolverProtectedOwner;
    fn wrapper_resolver_protected_setup(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, request: r::protected_call::InstructionRequest, returned: &mut Option<r::protected_call::InstructionReturned>) -> Result<(), Self::Error>;
    fn wrapper_resolver_protected_call(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, request: r::protected_call::CallRequest, returned: &mut Option<r::protected_call::Exit<Self::WrapperResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
impl<S: ResolverProtectedServices> ic::ResolverProtectedServices for Scope<S> {
    type InitializerResolverProtectedOwner = S::WrapperResolverProtectedOwner;
    fn initialization_resolver_protected_setup(&mut self, o: &mut ic::ResolverResources<'_, Self>, request: r::protected_call::InstructionRequest, returned: &mut Option<r::protected_call::InstructionReturned>) -> Result<(), Self::Error> {
        self.services.wrapper_resolver_protected_setup(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, request, returned)
    }
    fn initialization_resolver_protected_call(&mut self, o: &mut ic::ResolverResources<'_, Self>, request: r::protected_call::CallRequest, returned: &mut Option<r::protected_call::Exit<Self::InitializerResolverProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.wrapper_resolver_protected_call(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, request, returned)
    }
}

pub trait ResolverResolutionServices: ResolverProtectedServices {
    fn retain_wrapper_resolver_resolution(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, history: r::protected_call::resolution::History, held: &mut Option<r::protected_call::Exit<Self::WrapperResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
impl<S: ResolverResolutionServices> ic::ResolverResolutionServices for Scope<S> {
    fn retain_initialization_resolver_resolution(&mut self, o: &mut ic::ResolverResources<'_, Self>, history: r::protected_call::resolution::History, held: &mut Option<r::protected_call::Exit<Self::InitializerResolverProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.retain_wrapper_resolver_resolution(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, history, held)
    }
}

pub trait ResolverLoadServices: ResolverProtectedServices {
    fn retain_wrapper_resolver_load_exit(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, history: r::protected_call::load_publication::History, held: &mut Option<r::protected_call::Exit<Self::WrapperResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
impl<S: ResolverLoadServices> ic::ResolverLoadServices for Scope<S> {
    fn retain_initialization_resolver_load_exit(&mut self, o: &mut ic::ResolverResources<'_, Self>, history: r::protected_call::load_publication::History, held: &mut Option<r::protected_call::Exit<Self::InitializerResolverProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.retain_wrapper_resolver_load_exit(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, history, held)
    }
}

pub trait ResolverPriorCleanupServices: ResolverProtectedServices {
    fn retain_wrapper_resolver_prior_cleanup_exit(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, history: r::protected_call::prior_cleanup::History, held: &mut Option<r::protected_call::Exit<Self::WrapperResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
impl<S: ResolverPriorCleanupServices> ic::ResolverPriorCleanupServices for Scope<S> {
    fn retain_initialization_resolver_prior_cleanup_exit(&mut self, o: &mut ic::ResolverResources<'_, Self>, history: r::protected_call::prior_cleanup::History, held: &mut Option<r::protected_call::Exit<Self::InitializerResolverProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.retain_wrapper_resolver_prior_cleanup_exit(o.wrapper.storage.resources(), &mut o.wrapper.call, &mut o.wrapper.tail, &mut *o.calls, &mut *o.tail, history, held)
    }
}
