use super::{n, Scope, Services, Resources};
use n::{initialization as i, initialization_call as ic};
pub trait InitializationServices: Services {
    type WrapperInitializationCallOwner; type WrapperInitializationTailOwner;

    fn wrapper_initialization_sub(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, returned: &mut Option<i::StackAdjustment>) -> Result<(), Self::Error>;
    fn wrapper_initialization_write(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, request: i::WriteRequest, returned: &mut Option<i::WriteReturned>) -> Result<(), Self::Error>;
    fn wrapper_initialization_call(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, request: i::CallRequest) -> Result<(), Self::Error>;
    fn wrapper_initialization_add(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, returned: &mut Option<i::StackAdjustment>) -> Result<(), Self::Error>;
    fn wrapper_initialization_tail(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, calls: &mut i::Calls<Self::WrapperInitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::WrapperInitializationTailOwner>>, request: i::TailRequest) -> Result<(), Self::Error>;

    fn adopt_completed_wrapper_initialization(&mut self, r: Resources<'_, Self>, wrapper_call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, wrapper_tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, history: ic::History, held: &mut Option<ic::Retained<Self::WrapperInitializationCallOwner, Self::WrapperInitializationTailOwner>>) -> Result<(), Self::Error>;
}
impl<S: InitializationServices> ic::Services for Scope<S> {
    type InitializationCallOwner = S::WrapperInitializationCallOwner; type InitializationTailOwner = S::WrapperInitializationTailOwner;
    fn initialization_sub(&mut self, o: &mut n::Owners<Self>, calls: &mut i::Calls<Self::InitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::InitializationTailOwner>>, returned: &mut Option<i::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.wrapper_initialization_sub(o.storage.resources(), &mut o.call, &mut o.tail, calls, tail, returned)
    }
    fn initialization_write(&mut self, o: &mut n::Owners<Self>, calls: &mut i::Calls<Self::InitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::InitializationTailOwner>>, request: i::WriteRequest, returned: &mut Option<i::WriteReturned>) -> Result<(), Self::Error> {
        self.services.wrapper_initialization_write(o.storage.resources(), &mut o.call, &mut o.tail, calls, tail, request, returned)
    }
    fn initialization_call(&mut self, o: &mut n::Owners<Self>, calls: &mut i::Calls<Self::InitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::InitializationTailOwner>>, request: i::CallRequest) -> Result<(), Self::Error> {
        self.services.wrapper_initialization_call(o.storage.resources(), &mut o.call, &mut o.tail, calls, tail, request)
    }
    fn initialization_add(&mut self, o: &mut n::Owners<Self>, calls: &mut i::Calls<Self::InitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::InitializationTailOwner>>, returned: &mut Option<i::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.wrapper_initialization_add(o.storage.resources(), &mut o.call, &mut o.tail, calls, tail, returned)
    }
    fn initialization_tail(&mut self, o: &mut n::Owners<Self>, calls: &mut i::Calls<Self::InitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::InitializationTailOwner>>, request: i::TailRequest) -> Result<(), Self::Error> {
        self.services.wrapper_initialization_tail(o.storage.resources(), &mut o.call, &mut o.tail, calls, tail, request)
    }
    fn adopt_completed_initialization(&mut self, o: &mut n::Owners<Self>, history: ic::History, held: &mut Option<ic::Retained<Self::InitializationCallOwner, Self::InitializationTailOwner>>) -> Result<(), Self::Error> {
        self.services.adopt_completed_wrapper_initialization(o.storage.resources(), &mut o.call, &mut o.tail, history, held)
    }
}

