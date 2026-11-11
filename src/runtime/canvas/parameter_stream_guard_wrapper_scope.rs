use super::{n, r, ParentState, Retained, History};
use super::super as g;
pub struct Resources<'a, S: Services> { pub guard: &'a mut g::Owners<S>,

    pub source_tail: Option<&'a mut Option<r::TailCallReturned<S::TailOwner>>> }
pub trait Services: r::Services {
    type WrapperCallOwner; type WrapperTailOwner;

    fn wrapper_sub(&mut self, r: Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error>;
    fn wrapper_call(&mut self, r: Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, request: n::CallRequest) -> Result<(), Self::Error>;
    fn wrapper_lea(&mut self, r: Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, pc: u32, address: u64, returned: &mut Option<n::LeaReturned>) -> Result<(), Self::Error>;
    fn wrapper_add(&mut self, r: Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error>;
    fn wrapper_read_target(&mut self, r: Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, pc: u32, address: u64) -> Result<u64, Self::Error>;
    fn wrapper_tail(&mut self, r: Resources<'_, Self>, call: &mut Option<n::CallReturned<Self::WrapperCallOwner>>, tail: &mut Option<n::TailReturned<Self::WrapperTailOwner>>, request: n::TailRequest) -> Result<(), Self::Error>;

    fn adopt_completed_wrapper(&mut self, r: Resources<'_, Self>, history: History,
        held: &mut Option<Retained<Self::WrapperCallOwner, Self::WrapperTailOwner>>) -> Result<(), Self::Error>;
}
pub struct Scope<S: Services> { pub(super) services: S }
impl<S: Services> n::Services for Scope<S> {
    type Storage = ParentState<S>; type CallOwner = S::WrapperCallOwner; type TailOwner = S::WrapperTailOwner; type Error = S::Error;
    fn sub_rsp28(&mut self, o: &mut n::Owners<Self>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.wrapper_sub(o.storage.resources(), &mut o.call, &mut o.tail, returned)
    }
    fn call_58eca8(&mut self, o: &mut n::Owners<Self>, request: n::CallRequest) -> Result<(), Self::Error> {
        self.services.wrapper_call(o.storage.resources(), &mut o.call, &mut o.tail, request)
    }
    fn lea_rcx(&mut self, o: &mut n::Owners<Self>, pc: u32, address: u64, returned: &mut Option<n::LeaReturned>) -> Result<(), Self::Error> {
        self.services.wrapper_lea(o.storage.resources(), &mut o.call, &mut o.tail, pc, address, returned)
    }
    fn add_rsp28(&mut self, o: &mut n::Owners<Self>, returned: &mut Option<n::StackAdjustment>) -> Result<(), Self::Error> {
        self.services.wrapper_add(o.storage.resources(), &mut o.call, &mut o.tail, returned)
    }
    fn read_target(&mut self, o: &mut n::Owners<Self>, pc: u32, address: u64) -> Result<u64, Self::Error> {
        self.services.wrapper_read_target(o.storage.resources(), &mut o.call, &mut o.tail, pc, address)
    }
    fn tail(&mut self, o: &mut n::Owners<Self>, request: n::TailRequest) -> Result<(), Self::Error> {
        self.services.wrapper_tail(o.storage.resources(), &mut o.call, &mut o.tail, request)
    }
}
