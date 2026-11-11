use super::{i, ParentState, Retained, History};
use super::super as n;
pub trait Services: n::Services {
    type InitializationCallOwner; type InitializationTailOwner;

    fn initialization_sub(&mut self, o: &mut n::Owners<Self>, calls: &mut i::Calls<Self::InitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::InitializationTailOwner>>, returned: &mut Option<i::StackAdjustment>) -> Result<(), Self::Error>;
    fn initialization_write(&mut self, o: &mut n::Owners<Self>, calls: &mut i::Calls<Self::InitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::InitializationTailOwner>>, request: i::WriteRequest, returned: &mut Option<i::WriteReturned>) -> Result<(), Self::Error>;
    fn initialization_call(&mut self, o: &mut n::Owners<Self>, calls: &mut i::Calls<Self::InitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::InitializationTailOwner>>, request: i::CallRequest) -> Result<(), Self::Error>;
    fn initialization_add(&mut self, o: &mut n::Owners<Self>, calls: &mut i::Calls<Self::InitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::InitializationTailOwner>>, returned: &mut Option<i::StackAdjustment>) -> Result<(), Self::Error>;
    fn initialization_tail(&mut self, o: &mut n::Owners<Self>, calls: &mut i::Calls<Self::InitializationCallOwner>, tail: &mut Option<i::TailReturned<Self::InitializationTailOwner>>, request: i::TailRequest) -> Result<(), Self::Error>;

    fn adopt_completed_initialization(&mut self, o: &mut n::Owners<Self>, history: History,
        held: &mut Option<Retained<Self::InitializationCallOwner, Self::InitializationTailOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> i::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::InitializationCallOwner; type TailOwner = S::InitializationTailOwner; type Error = S::Error;
    fn sub_rsp28(&mut self, o: &mut i::Owners<Self>, returned: &mut Option<i::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.initialization_sub(&mut o.storage.owners, &mut o.calls, &mut o.tail, returned)
    }
    fn write_argument(&mut self, o: &mut i::Owners<Self>, request: i::WriteRequest, returned: &mut Option<i::WriteReturned>) -> Result<(), Self::Error> {
        self.services.initialization_write(&mut o.storage.owners, &mut o.calls, &mut o.tail, request, returned)
    }
    fn call_58ee04(&mut self, o: &mut i::Owners<Self>, request: i::CallRequest) -> Result<(), Self::Error> {
        self.services.initialization_call(&mut o.storage.owners, &mut o.calls, &mut o.tail, request)
    }
    fn add_rsp28(&mut self, o: &mut i::Owners<Self>, returned: &mut Option<i::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.initialization_add(&mut o.storage.owners, &mut o.calls, &mut o.tail, returned)
    }
    fn tail_58ee04(&mut self, o: &mut i::Owners<Self>, request: i::TailRequest) -> Result<(), Self::Error> {
        self.services.initialization_tail(&mut o.storage.owners, &mut o.calls, &mut o.tail, request)
    }
}
