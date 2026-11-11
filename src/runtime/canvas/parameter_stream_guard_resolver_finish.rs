use super::{r, i, Scope, Services, History, Retained, Request, Stage};
pub enum Supplied<C, T> { Call(i::CallReturned<C>), Tail(i::TailReturned<T>) }
pub enum ReturnFault<E> { ReturnBinding, OccupiedCall, OccupiedTail, Parent(i::Fault<E>), Adoption(E), ResourcesNotAdopted }
pub struct ReturnRejected<S: Services> { pub parent: i::Parts<S>, pub supplied: Option<Supplied<S::CallOwner, S::TailOwner>>,
    pub retained: Option<Retained>, pub history: History, pub fault: ReturnFault<S::Error> }
impl<S: Services> r::native_return::Returned<Scope<S>> {
    pub fn into_initializer_parent(self, supplied: Supplied<S::CallOwner, S::TailOwner>) -> Result<i::Prepared<S>, ReturnRejected<S>> {
        let actual = *self.actual_return(); let native = self.history(); let parts = self.into_parts(); let state = parts.body.storage;
        let history = History { actual: state.binding, request: state.request, body: parts.body.captures, native_return: native };
        let mut held = Some(Retained { actual: state.binding, request: state.request, body: parts.body.captures,
            stage: Stage::NativeReturn { phase: parts.phase, index: parts.index, attempted: parts.attempted, stopped: parts.stopped }, native_return: Some(native) });
        let mut parent = state.into_parent(parts.body.services.services);
        let binding = match (history.request, &supplied) {
            (Request::Call(request), Supplied::Call(a)) => actual.entry_rsp == request.entry_rsp
                && actual.after_rsp == request.work_rsp && actual.target == parent.actual.source_image_base.wrapping_add(u64::from(request.expected_resume_pc))
                && a.actual.resume_pc == request.expected_resume_pc && a.actual.work_rsp == request.work_rsp && a.actual.rax == actual.rax,
            (Request::Tail(request), Supplied::Tail(a)) => actual.entry_rsp == request.entry_rsp && actual.after_rsp == request.entry_rsp.wrapping_add(8)
                && a.actual.entry_rsp == actual.entry_rsp && a.actual.after_rsp == actual.after_rsp && a.actual.target == actual.target && a.actual.rax == actual.rax,
            _ => false,
        };
        if !binding { return Err(ReturnRejected { parent: parent.into_parts(), supplied: Some(supplied), retained: held, history, fault: ReturnFault::ReturnBinding }); }
        match (history.request, supplied) {
            (Request::Call(request), Supplied::Call(a)) => {
                if parent.owners.calls[request.index].is_some() { return Err(ReturnRejected { parent: parent.into_parts(), supplied: Some(Supplied::Call(a)), retained: held, history, fault: ReturnFault::OccupiedCall }); }
                parent.owners.calls[request.index] = Some(a);
                if let Err(e) = parent.record_resolver_call_return(request) { return Err(ReturnRejected { parent: parent.into_parts(), supplied: None, retained: held, history, fault: ReturnFault::Parent(e) }); }
            }
            (Request::Tail(request), Supplied::Tail(a)) => {
                if parent.owners.tail.is_some() { return Err(ReturnRejected { parent: parent.into_parts(), supplied: Some(Supplied::Tail(a)), retained: held, history, fault: ReturnFault::OccupiedTail }); }
                parent.owners.tail = Some(a);
                if let Err(e) = parent.record_resolver_tail_return(request) { return Err(ReturnRejected { parent: parent.into_parts(), supplied: None, retained: held, history, fault: ReturnFault::Parent(e) }); }
            }

            (_, other) => return Err(ReturnRejected { parent: parent.into_parts(), supplied: Some(other), retained: held, history, fault: ReturnFault::ReturnBinding }),
        }
        if let Err(e) = parent.services.adopt_completed_resolver(&mut parent.owners, history, &mut held) {
            return Err(ReturnRejected { parent: parent.into_parts(), supplied: None, retained: held, history, fault: ReturnFault::Adoption(e) });
        }
        if held.is_some() { return Err(ReturnRejected { parent: parent.into_parts(), supplied: None, retained: held, history, fault: ReturnFault::ResourcesNotAdopted }); }
        parent.captures.resolver_calls[history.request.index()] = Some(history);
        parent.attempted = None; parent.stopped = false; Ok(parent)
    }
}
