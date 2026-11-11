use super::{fallback as f, CallSite, CallRequest, Target};
pub trait Services: super::Services {

    fn fallback_load_current(&mut self, owners: &mut super::Owners<Self>, request: f::LoadRequest,
        returned: &mut Option<f::LoadReturned>) -> Result<(), Self::Error>;
    fn fallback_actual_return(&mut self, owners: &mut super::Owners<Self>, request: f::ReturnRequest,
        returned: &mut Option<f::ActualReturn>) -> Result<(), Self::Error>;
}
struct Borrowed<'a, S: Services> { services: &'a mut S, owners: &'a mut super::Owners<S> }
impl<S: Services> f::Services for Borrowed<'_, S> {
    type Error = S::Error;
    fn load_current(&mut self, request: f::LoadRequest, returned: &mut Option<f::LoadReturned>) -> Result<(), Self::Error> {
        self.services.fallback_load_current(self.owners, request, returned)
    }
    fn actual_return(&mut self, request: f::ReturnRequest, returned: &mut Option<f::ActualReturn>) -> Result<(), Self::Error> {
        self.services.fallback_actual_return(self.owners, request, returned)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ActualCall { pub call_pc: u32, pub caller_rsp: u64, pub child: f::ActualEntry }
#[derive(Clone, Copy, Debug)]
pub struct History { pub actual: ActualCall, pub request: CallRequest, pub leaf: f::History }
pub enum Admission<E> { NotPending, Request(super::Fault<E>), OccupiedCall, CallBinding, Entry }
pub struct Rejected<S: Services> { pub parent: super::Prepared<S>, pub actual: ActualCall, pub fault: Admission<S::Error> }
pub struct Bound<S: Services> { parent: super::Prepared<S>, actual: ActualCall, request: CallRequest, cursor: f::Cursor }
pub struct Preserved<S: Services> { pub parent: super::Parts<S>, pub actual: ActualCall, pub request: CallRequest, pub leaf: f::Parts }
pub struct Stopped<S: Services> { pub pending: Bound<S>, pub fault: f::Fault<S::Error> }
pub enum Poll<S: Services> { Pending(Bound<S>), Ready(Returned<S>), Failed(Stopped<S>) }
pub struct Returned<S: Services> { bound: Bound<S> }
impl<S: Services> super::Prepared<S> {
    pub fn bind_actual_fallback_entry(mut self, actual: ActualCall) -> Result<Bound<S>, Rejected<S>> {
        let old = self.stopped; self.stopped = true;
        if old || self.phase != super::Phase::Fallback { return Err(Rejected { parent: self, actual, fault: Admission::NotPending }); }
        let request = match self.fallback_request() {
            Ok(request) => request, Err(e) => return Err(Rejected { parent: self, actual, fault: Admission::Request(e) }),
        };
        self.captures.requests[request.site.index()] = Some(request);
        if self.owners.calls[request.site.index()].is_some() { return Err(Rejected { parent: self, actual, fault: Admission::OccupiedCall }); }
        if actual.call_pc != request.site.pc() || actual.caller_rsp != request.work_rsp
            || actual.child.pc != 0x55e7ac || actual.child.source_image_base != self.actual.source_image_base
            || actual.child.rsp != actual.caller_rsp.wrapping_sub(8)
            || !matches!(request.target, Target::DirectRva(0x55e7ac)) {
            return Err(Rejected { parent: self, actual, fault: Admission::CallBinding });
        }
        let cursor = match f::Cursor::from_actual_entry(actual.child) {
            Ok(cursor) => cursor, Err(_) => return Err(Rejected { parent: self, actual, fault: Admission::Entry }),
        };
        Ok(Bound { parent: self, actual, request, cursor })
    }
}
impl<S: Services> Bound<S> {
    pub fn poll(mut self, budget: usize) -> Poll<S> {
        let result = { let mut borrowed = Borrowed { services: &mut self.parent.services, owners: &mut self.parent.owners };
            self.cursor.poll(&mut borrowed, budget) };
        match result { Ok(false) => Poll::Pending(self), Ok(true) => Poll::Ready(Returned { bound: self }),
            Err(fault) => Poll::Failed(Stopped { pending: self, fault }) }
    }
    pub fn into_preserved_lookup(self) -> Preserved<S> { Preserved { parent: self.parent.into_parts(),
        actual: self.actual, request: self.request, leaf: self.cursor.into_parts() } }
}
pub enum ReturnFault<E> { MissingReceipt, ReturnBinding, OccupiedCall, Parent(super::Fault<E>) }
pub struct ReturnRejected<S: Services> { pub pending: Preserved<S>, pub supplied: Option<super::CallReturned<S::CallOwner>>, pub fault: ReturnFault<S::Error> }
impl<S: Services> Returned<S> {
    pub fn into_lookup_parent(self, supplied: super::CallReturned<S::CallOwner>) -> Result<super::Prepared<S>, ReturnRejected<S>> {
        let mut bound = self.bound;
        let returned = match bound.cursor.captures().actual_return {
            Some(receipt) => receipt,
            None => return Err(ReturnRejected { pending: bound.into_preserved_lookup(), supplied: Some(supplied), fault: ReturnFault::MissingReceipt }),
        };
        let actual = supplied.actual;
        if returned.target != bound.parent.actual.source_image_base.wrapping_add(u64::from(CallSite::Fallback.resume_pc()))
            || returned.after_rsp != bound.actual.caller_rsp || actual.resume_pc != CallSite::Fallback.resume_pc()
            || actual.work_rsp != bound.actual.caller_rsp || actual.rax != returned.rax {
            return Err(ReturnRejected { pending: bound.into_preserved_lookup(), supplied: Some(supplied), fault: ReturnFault::ReturnBinding });
        }
        let slot = bound.request.site.index();
        if bound.parent.owners.calls[slot].is_some() {
            return Err(ReturnRejected { pending: bound.into_preserved_lookup(), supplied: Some(supplied), fault: ReturnFault::OccupiedCall });
        }
        bound.parent.owners.calls[slot] = Some(supplied);
        let word = match bound.parent.record_call_return(bound.request) {
            Ok(word) => word,
            Err(e) => return Err(ReturnRejected { pending: bound.into_preserved_lookup(), supplied: None, fault: ReturnFault::Parent(e) }),
        };
        bound.parent.captures.fallback_call = Some(History { actual: bound.actual, request: bound.request, leaf: bound.cursor.history() });
        bound.parent.record_fallback_value(word);
        bound.parent.attempted = None; bound.parent.stopped = false;
        Ok(bound.parent)
    }
}
