use super::crt_query_calloc::{SourceCrtQueryCalloc58f074, SourceCrtQueryCallocProgress,
    SourceCrtQueryCallocRequest, SourceCrtQueryCallocReceipt, SourceCrtQueryCallocError};
use super::crt_storage_native_return::SourceCrtNativeRetFrame;
use super::crt_storage_owned_free::SourceCrtStorageActualAllocation;

#[must_use = "retain actual physical H on every rejected protected continuation"]
pub enum SourceCrtCallocPhysicalReturn<A> { Null, Allocated(A) }
pub struct SourceCrtCallocAllocationBindings<A, P, S, R> {

    pub describe_allocation: fn(&A) -> SourceCrtStorageActualAllocation,
    pub actual_frame: fn(&P, &S) -> SourceCrtNativeRetFrame,
    pub actual_rbx: fn(&P, &S) -> u64,
    pub owned_receipt: fn(&P, &S) -> Option<SourceCrtQueryCallocReceipt>,
    pub actual_receipt: fn(&R) -> SourceCrtQueryCallocReceipt,
}
#[must_use = "retain actual physical H/protected continuation/full frame/FP/native resources"]
pub struct SourceCrtCallocAllocationCapsule<A, P, S, R> {
    pub physical_return: SourceCrtCallocPhysicalReturn<A>,
    pub prefix: P,
    pub services: S,
    pub actual_receipt: R,
    pub bindings: SourceCrtCallocAllocationBindings<A, P, S, R>,
}
#[must_use = "retain physical allocation or actual NULL prefix through caller's next source step"]
pub struct SourceCrtCallocAllocationAccepted<A, P, S, R> {
    pub actual: SourceCrtCallocAllocationCapsule<A, P, S, R>,
    pub source: SourceCrtQueryCallocProgress,
}
#[must_use = "retain entire rejected allocation owner and stopped original calloc"]
pub struct SourceCrtCallocAllocationError<A, P, S, R> {
    pub actual: SourceCrtCallocAllocationCapsule<A, P, S, R>,
    pub pending: SourceCrtQueryCallocProgress,
    pub allocation: Option<SourceCrtStorageActualAllocation>,
    pub observed: SourceCrtQueryCallocReceipt,
    pub owned_observed: Option<SourceCrtQueryCallocReceipt>,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub actual_rbx: u64,
    pub source: Option<SourceCrtQueryCallocError>,
}
impl SourceCrtQueryCalloc58f074 {

    pub fn accept_owned_protected_allocation<A, P, S, R>(&mut self,
        actual: SourceCrtCallocAllocationCapsule<A, P, S, R>)
        -> Result<SourceCrtCallocAllocationAccepted<A, P, S, R>, SourceCrtCallocAllocationError<A, P, S, R>> {
        self.stop();
        let pending = self.progress();
        let allocation = match &actual.physical_return {
            SourceCrtCallocPhysicalReturn::Null => None,
            SourceCrtCallocPhysicalReturn::Allocated(owner) => Some((actual.bindings.describe_allocation)(owner)),
        };
        let observed = (actual.bindings.actual_receipt)(&actual.actual_receipt);
        let owned_observed = (actual.bindings.owned_receipt)(&actual.prefix, &actual.services);
        let actual_frame = (actual.bindings.actual_frame)(&actual.prefix, &actual.services);
        let actual_rbx = (actual.bindings.actual_rbx)(&actual.prefix, &actual.services);
        let matches = match (pending, observed) {
            (SourceCrtQueryCallocProgress::Request(SourceCrtQueryCallocRequest::RequiredProtected(expected)),
                SourceCrtQueryCallocReceipt::ProtectedContinued { request, actual_frame: returned, actual_rbx: returned_rbx, actual_extra_push }) => {
                let physical_matches = match allocation {
                    None => returned.actual_rax == 0,
                    Some(descriptor) => descriptor.actual_base_word.get() == returned.actual_rax
                        && descriptor.actual_accessible_bytes >= expected.bytes_r8,
                };
                request == expected && owned_observed == Some(observed)
                    && actual_frame == returned && actual_rbx == returned_rbx
                    && actual_rbx == expected.bytes_r8
                    && returned.actual_rip == expected.required_actual_resume
                    && returned.actual_rsp == expected.required_resume_rsp
                    && actual_extra_push == expected.actual_extra_push && physical_matches
            }
            _ => false,
        };
        if !matches {
            return Err(SourceCrtCallocAllocationError { actual, pending, allocation, observed,
                owned_observed, actual_frame, actual_rbx, source: None });
        }
        match self.accept_actual_completion(observed) {
            Ok(source) => Ok(SourceCrtCallocAllocationAccepted { actual, source }),
            Err(source) => Err(SourceCrtCallocAllocationError { actual, pending, allocation, observed,
                owned_observed, actual_frame, actual_rbx, source: Some(source) }),
        }
    }
}
