use super::{i, Scope};
use i::resolver_prefix::{self as r, selected_call as c};
use super::super as n;
pub struct ResolverResources<'a, S: ResolverServices> { pub wrapper: &'a mut n::Owners<S>,
    pub calls: &'a mut i::Calls<S::InitializationCallOwner>, pub tail: &'a mut Option<i::TailReturned<S::InitializationTailOwner>> }
pub trait ResolverServices: super::Services {
    fn initialization_resolver_read_word(&mut self, o: &mut ResolverResources<'_, Self>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn initialization_resolver_read_dword(&mut self, o: &mut ResolverResources<'_, Self>, pc: u32, address: u64) -> Result<u32, Self::Error>;
    fn initialization_resolver_xchg(&mut self, o: &mut ResolverResources<'_, Self>, pc: u32, address: u64, word: u64, returned: &mut Option<r::ExchangeReturned>) -> Result<(), Self::Error>;
    fn initialization_resolver_restore(&mut self, o: &mut ResolverResources<'_, Self>, pc: u32, register: r::native_return::Register, address: u64, returned: &mut Option<r::native_return::Restored>) -> Result<(), Self::Error>;
    fn initialization_resolver_add(&mut self, o: &mut ResolverResources<'_, Self>, returned: &mut Option<r::StackAdjustment>) -> Result<(), Self::Error>;
    fn initialization_resolver_pop(&mut self, o: &mut ResolverResources<'_, Self>, pc: u32, register: r::native_return::Register, returned: &mut Option<r::Pop>) -> Result<(), Self::Error>;
    fn initialization_resolver_ret(&mut self, o: &mut ResolverResources<'_, Self>, returned: &mut Option<r::native_return::ActualReturn>) -> Result<(), Self::Error>;

    fn adopt_initialization_resolver(&mut self, o: &mut ResolverResources<'_, Self>, history: c::History, held: &mut Option<c::Retained>) -> Result<(), Self::Error>;
}
pub trait ResolverEntryServices: ResolverServices {
    fn initialization_resolver_entry(&mut self, o: &mut ResolverResources<'_, Self>, request: r::native_entry::Request,
        returned: &mut Option<r::native_entry::InstructionReturned>) -> Result<(), Self::Error>;
}
pub trait ResolverProtectedServices: ResolverServices {
    type InitializerResolverProtectedOwner;
    fn initialization_resolver_protected_setup(&mut self, o: &mut ResolverResources<'_, Self>, request: r::protected_call::InstructionRequest,
        returned: &mut Option<r::protected_call::InstructionReturned>) -> Result<(), Self::Error>;
    fn initialization_resolver_protected_call(&mut self, o: &mut ResolverResources<'_, Self>, request: r::protected_call::CallRequest,
        returned: &mut Option<r::protected_call::Exit<Self::InitializerResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
pub trait ResolverResolutionServices: ResolverProtectedServices {
    fn retain_initialization_resolver_resolution(&mut self, o: &mut ResolverResources<'_, Self>, history: r::protected_call::resolution::History,
        held: &mut Option<r::protected_call::Exit<Self::InitializerResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
pub trait ResolverLoadServices: ResolverProtectedServices {
    fn retain_initialization_resolver_load_exit(&mut self, o: &mut ResolverResources<'_, Self>, history: r::protected_call::load_publication::History,
        held: &mut Option<r::protected_call::Exit<Self::InitializerResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
fn resources<S: ResolverServices>(o: &mut i::Owners<Scope<S>>) -> ResolverResources<'_, S> {
    ResolverResources { wrapper: &mut o.storage.owners, calls: &mut o.calls, tail: &mut o.tail }
}
impl<S: ResolverServices> c::Services for Scope<S> {
    fn resolver_read_word(&mut self, o: &mut i::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.initialization_resolver_read_word(&mut resources(o), pc, address)
    }
    fn resolver_read_dword(&mut self, o: &mut i::Owners<Self>, pc: u32, address: u64) -> Result<u32, Self::Error> {
        self.services.initialization_resolver_read_dword(&mut resources(o), pc, address)
    }
    fn resolver_xchg_word(&mut self, o: &mut i::Owners<Self>, pc: u32, address: u64, word: u64, returned: &mut Option<r::ExchangeReturned>) -> Result<(), Self::Error> {
        self.services.initialization_resolver_xchg(&mut resources(o), pc, address, word, returned)
    }
    fn resolver_restore_word(&mut self, o: &mut i::Owners<Self>, pc: u32, register: r::native_return::Register, address: u64, returned: &mut Option<r::native_return::Restored>) -> Result<(), Self::Error> {
        self.services.initialization_resolver_restore(&mut resources(o), pc, register, address, returned)
    }
    fn resolver_add_rsp20(&mut self, o: &mut i::Owners<Self>, returned: &mut Option<r::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.initialization_resolver_add(&mut resources(o), returned)
    }
    fn resolver_pop(&mut self, o: &mut i::Owners<Self>, pc: u32, register: r::native_return::Register, returned: &mut Option<r::Pop>) -> Result<(), Self::Error> {
        self.services.initialization_resolver_pop(&mut resources(o), pc, register, returned)
    }
    fn resolver_ret(&mut self, o: &mut i::Owners<Self>, returned: &mut Option<r::native_return::ActualReturn>) -> Result<(), Self::Error> {
        self.services.initialization_resolver_ret(&mut resources(o), returned)
    }
    fn adopt_completed_resolver(&mut self, o: &mut i::Owners<Self>, history: c::History, held: &mut Option<c::Retained>) -> Result<(), Self::Error> {
        self.services.adopt_initialization_resolver(&mut resources(o), history, held)
    }
}
impl<S: ResolverEntryServices> c::native_entry::Services for Scope<S> {
    fn resolver_prologue_instruction(&mut self, o: &mut i::Owners<Self>, request: r::native_entry::Request,
        returned: &mut Option<r::native_entry::InstructionReturned>) -> Result<(), Self::Error> {
        self.services.initialization_resolver_entry(&mut resources(o), request, returned)
    }
}
impl<S: ResolverProtectedServices> c::protected_scope::Services for Scope<S> {
    type ResolverProtectedOwner = S::InitializerResolverProtectedOwner;
    fn resolver_protected_setup(&mut self, o: &mut i::Owners<Self>, request: r::protected_call::InstructionRequest,
        returned: &mut Option<r::protected_call::InstructionReturned>) -> Result<(), Self::Error> {
        self.services.initialization_resolver_protected_setup(&mut resources(o), request, returned)
    }
    fn resolver_protected_call(&mut self, o: &mut i::Owners<Self>, request: r::protected_call::CallRequest,
        returned: &mut Option<r::protected_call::Exit<Self::ResolverProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.initialization_resolver_protected_call(&mut resources(o), request, returned)
    }
}
impl<S: ResolverResolutionServices> c::protected_scope::ResolutionServices for Scope<S> {
    fn retain_resolver_resolution(&mut self, o: &mut i::Owners<Self>, history: r::protected_call::resolution::History,
        held: &mut Option<r::protected_call::Exit<Self::ResolverProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.retain_initialization_resolver_resolution(&mut resources(o), history, held)
    }
}
impl<S: ResolverLoadServices> c::protected_scope::LoadServices for Scope<S> {
    fn retain_resolver_load_exit(&mut self, o: &mut i::Owners<Self>, history: r::protected_call::load_publication::History,
        held: &mut Option<r::protected_call::Exit<Self::ResolverProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.retain_initialization_resolver_load_exit(&mut resources(o), history, held)
    }
}

pub trait ResolverPriorCleanupServices: ResolverProtectedServices {
    fn retain_initialization_resolver_prior_cleanup_exit(&mut self, o: &mut ResolverResources<'_, Self>, history: r::protected_call::prior_cleanup::History,
        held: &mut Option<r::protected_call::Exit<Self::InitializerResolverProtectedOwner>>) -> Result<(), Self::Error>;
}
impl<S: ResolverPriorCleanupServices> c::protected_scope::PriorCleanupServices for Scope<S> {
    fn retain_resolver_prior_cleanup_exit(&mut self, o: &mut i::Owners<Self>, history: r::protected_call::prior_cleanup::History,
        held: &mut Option<r::protected_call::Exit<Self::ResolverProtectedOwner>>) -> Result<(), Self::Error> {
        self.services.retain_initialization_resolver_prior_cleanup_exit(&mut resources(o), history, held)
    }
}
