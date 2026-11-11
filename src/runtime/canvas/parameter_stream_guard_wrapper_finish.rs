use super::{n, r, Services, ParentParts, Supplied, Resumed, Retained, History, ReturnFault, Resources};
type Error<S: Services> = (ParentParts<S>, Option<Supplied<<S as super::super::Services>::CallOwner, <S as r::Services>::TailOwner>>, ReturnFault<<S as super::super::Services>::Error>);
pub(super) fn finish<S: Services>(parent: ParentParts<S>, supplied: Supplied<S::CallOwner, S::TailOwner>, actual: n::ActualTailReturn,
    history: History, held: &mut Option<Retained<S::WrapperCallOwner, S::WrapperTailOwner>>) -> Result<Resumed<S>, Error<S>> {
    match (parent, supplied) {
        (ParentParts::Call { parts, request: Some(request) }, Supplied::Call(supplied)) => {
            if actual.target != parts.actual.source_image_base.wrapping_add(request.site.resume_pc() as u64)
                || actual.after_rsp != request.work_rsp || supplied.actual.resume_pc != request.site.resume_pc()
                || supplied.actual.work_rsp != actual.after_rsp || supplied.actual.rax != actual.rax || supplied.actual.rbx != actual.rbx {
                return Err((ParentParts::Call { parts, request: Some(request) }, Some(Supplied::Call(supplied)), ReturnFault::ReturnBinding));
            }
            if parts.owners.call.is_some() { return Err((ParentParts::Call { parts, request: Some(request) }, Some(Supplied::Call(supplied)), ReturnFault::OccupiedCall)); }
            let mut p = super::super::Prepared { services: parts.services, owners: parts.owners, actual: parts.actual, captures: parts.captures,
                phase: parts.phase, attempted: parts.attempted, stopped: true };
            p.owners.call = Some(supplied);
            if let Err(e) = p.record_call_return(request) { return Err((ParentParts::Call { parts: p.into_parts(), request: Some(request) }, None, ReturnFault::Call(e))); }
            if let Err(e) = p.services.adopt_completed_wrapper(Resources { guard: &mut p.owners, source_tail: None }, history, held) {
                return Err((ParentParts::Call { parts: p.into_parts(), request: Some(request) }, None, ReturnFault::Adoption(e)));
            }
            if held.is_some() { return Err((ParentParts::Call { parts: p.into_parts(), request: Some(request) }, None, ReturnFault::ResourcesNotAdopted)); }
            p.captures.wrapper_call = Some(history); p.attempted = None; p.stopped = false; Ok(Resumed::Call(p))
        }
        (ParentParts::Tail { mut parts, request: Some(request) }, Supplied::Tail(supplied)) => {
            let a = supplied.actual;
            if a.entry_rsp != request.entry_rsp || a.entry_rsp != actual.entry_rsp || a.after_rsp != actual.after_rsp
                || a.target != actual.target || a.rax != actual.rax || a.rbx != actual.rbx {
                return Err((ParentParts::Tail { parts, request: Some(request) }, Some(Supplied::Tail(supplied)), ReturnFault::ReturnBinding));
            }
            if parts.tail.is_some() { return Err((ParentParts::Tail { parts, request: Some(request) }, Some(Supplied::Tail(supplied)), ReturnFault::OccupiedTail)); }
            parts.tail = Some(supplied); let mut p = r::Prepared::from_selected_parts(parts);
            if let Err(e) = p.record_selected_tail(request) { return Err((ParentParts::Tail { parts: p.into_parts(), request: Some(request) }, None, ReturnFault::Tail(e))); }
            let mut parts = p.into_parts();
            if let Err(e) = parts.body.services.adopt_completed_wrapper(Resources { guard: &mut parts.body.owners, source_tail: Some(&mut parts.tail) }, history, held) {
                return Err((ParentParts::Tail { parts, request: Some(request) }, None, ReturnFault::Adoption(e)));
            }
            if held.is_some() { return Err((ParentParts::Tail { parts, request: Some(request) }, None, ReturnFault::ResourcesNotAdopted)); }
            parts.captures.wrapper_tail = Some(history); parts.attempted = None; parts.stopped = false;
            Ok(Resumed::Tail(r::Prepared::from_selected_parts(parts)))
        }
        (parent, supplied) => Err((parent, Some(supplied), ReturnFault::WrongOrigin)),
    }
}
