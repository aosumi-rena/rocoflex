use super::crt_storage_owned_free::{SourceCrtStorageOwnedFreeCompletion,
    SourceCrtStorageActualDeleteCompletion, SourceCrtStorageOwnedFreeFailure,
    SourceCrtStorageOwnedFreeReceiptError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtNativeRetFrame {
    pub actual_rip: u64,
    pub actual_rsp: u64,
    pub actual_rax: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtNativeRetStackRead { pub actual_address: u64, pub actual_word: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtNativeRetired {
    pub before: SourceCrtNativeRetFrame,
    pub actual_stack_read: SourceCrtNativeRetStackRead,
    pub after: SourceCrtNativeRetFrame,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtNativeRetObservation {
    Retired(SourceCrtNativeRetired),

    Partial { before: SourceCrtNativeRetFrame, actual_stack_read: Option<SourceCrtNativeRetStackRead>,
        after: SourceCrtNativeRetFrame },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtNativeRetExpectation {
    pub before: SourceCrtNativeRetFrame,

    pub actual_required_target: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtNativeRetMismatch { BeforeFrame, StackReadAddress, ActualTarget, AfterFrame }

pub fn validate_source_crt_native_ret(expected: SourceCrtNativeRetExpectation, actual: SourceCrtNativeRetired)
    -> Result<(), SourceCrtNativeRetMismatch> {
    if actual.before != expected.before { return Err(SourceCrtNativeRetMismatch::BeforeFrame); }
    if actual.actual_stack_read.actual_address != expected.before.actual_rsp {
        return Err(SourceCrtNativeRetMismatch::StackReadAddress);
    }
    if actual.actual_stack_read.actual_word != expected.actual_required_target
        || actual.after.actual_rip != actual.actual_stack_read.actual_word {
        return Err(SourceCrtNativeRetMismatch::ActualTarget);
    }
    if actual.after.actual_rsp != expected.before.actual_rsp.wrapping_add(8)
        || actual.after.actual_rax != expected.before.actual_rax {
        return Err(SourceCrtNativeRetMismatch::AfterFrame);
    }
    Ok(())
}

pub struct SourceCrtStorageNativeRetBindings<D, P, T> {
    pub delete_completion: fn(&D) -> SourceCrtStorageActualDeleteCompletion,
    pub current_frame: fn(&P) -> SourceCrtNativeRetFrame,
    pub expected_target: fn(&T) -> u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtStorageNativeRetBindReason { CompletedFreeRequired, DeleteOwnerRequired, ActualBeforeRetFrameRequired }
#[must_use = "retain same D/H/disposition/frame/FP/services and independent target owner"]
pub struct SourceCrtStorageNativeRetBindFailure<D, P, S, T> {
    pub owner: SourceCrtStorageOwnedFreeCompletion<D, P, S>,
    pub target_owner: T,
    pub reason: SourceCrtStorageNativeRetBindReason,
    pub expected: SourceCrtNativeRetExpectation,
    pub actual_frame: SourceCrtNativeRetFrame,
    pub actual_delete: SourceCrtStorageActualDeleteCompletion,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtNativeRetPhase { ActualReceiptRequired, ActualContinuationRequired }
pub type SourceCrtStorageNativeRetPhase = SourceCrtNativeRetPhase;
pub enum SourceCrtNativeRetError<N, E> {
    Adapter { actual: N, observation: SourceCrtNativeRetObservation, cause: E },
    Partial { actual: N, observation: SourceCrtNativeRetObservation },
    ReceiptChanged { actual: N, before: SourceCrtNativeRetObservation, after: SourceCrtNativeRetObservation },
    Mismatch { actual: N, observation: SourceCrtNativeRetired, cause: SourceCrtNativeRetMismatch },
    PrefixMismatch { actual: N, observation: SourceCrtNativeRetired, actual_frame: SourceCrtNativeRetFrame },
}
pub type SourceCrtStorageNativeRetError<N, E> = SourceCrtNativeRetError<N, E>;
impl<N, E> SourceCrtNativeRetError<N, E> {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Adapter { .. } => "actual CRT near-RET adapter failed; same owner/target/frame/FP/resources retained",
            Self::Partial { .. } => "CRT near RET has only an actual partial/fault prefix; retired return remains required",
            Self::ReceiptChanged { .. } => "actual RET observation changed while applying its receipt; both observations and advanced state retained",
            Self::Mismatch { .. } => "actual CRT retired frame/stack read/target/RAX differs from its independent source binding",
            Self::PrefixMismatch { .. } => "actual CRT retired receipt differs from the same advanced owned frame",
        }
    }
}

pub type SourceCrtNativeReturnFailure<O, E> = SourceCrtStorageOwnedFreeFailure<O, E>;

pub fn accept_source_crt_native_ret_receipt<O, N, E, A, F>(owner: &mut O,
    expected: SourceCrtNativeRetExpectation, mut actual: N,
    project: fn(&N) -> SourceCrtNativeRetObservation, apply: A, current_frame: F)
    -> Result<(N, SourceCrtNativeRetired), SourceCrtNativeRetError<N, E>>
where A: FnOnce(&mut O, &mut N) -> Result<(), E>, F: FnOnce(&O) -> SourceCrtNativeRetFrame {
    let observation = project(&actual);
    if let Err(cause) = apply(owner, &mut actual) {
        return Err(SourceCrtNativeRetError::Adapter { actual, observation, cause });
    }
    let after = project(&actual);
    if after != observation { return Err(SourceCrtNativeRetError::ReceiptChanged { actual, before: observation, after }); }
    let SourceCrtNativeRetObservation::Retired(retired) = observation else {
        return Err(SourceCrtNativeRetError::Partial { actual, observation });
    };
    if let Err(cause) = validate_source_crt_native_ret(expected, retired) {
        return Err(SourceCrtNativeRetError::Mismatch { actual, observation: retired, cause });
    }
    let actual_frame = current_frame(owner);
    if actual_frame != retired.after {
        return Err(SourceCrtNativeRetError::PrefixMismatch { actual, observation: retired, actual_frame });
    }
    Ok((actual, retired))
}
#[must_use = "retain SAME completed D/H/disposition/P/S, target record and actual native return"]
pub struct SourceCrtStorageCallbackReturned<D, P, S, T, N> {
    pub owner: SourceCrtStorageOwnedFreeCompletion<D, P, S>,
    pub target_owner: T,
    pub expected: SourceCrtNativeRetExpectation,
    pub actual_delete: SourceCrtStorageActualDeleteCompletion,
    pub actual_retired: SourceCrtNativeRetired,
    pub actual_return: N,
}
pub struct SourceCrtStorageNativeRetParts<D, P, S, T> {
    pub owner: SourceCrtStorageOwnedFreeCompletion<D, P, S>,
    pub target_owner: T,
    pub expected: SourceCrtNativeRetExpectation,
    pub actual_delete: SourceCrtStorageActualDeleteCompletion,
    pub phase: SourceCrtStorageNativeRetPhase,
    pub current_frame: fn(&P) -> SourceCrtNativeRetFrame,
}

#[must_use = "retain SAME completed free and actual return/exception continuation"]
pub struct SourceCrtStorageCallbackNativeRet<D, P, S, T> {
    owner: SourceCrtStorageOwnedFreeCompletion<D, P, S>,
    target_owner: T,
    expected: SourceCrtNativeRetExpectation,
    actual_delete: SourceCrtStorageActualDeleteCompletion,
    phase: SourceCrtStorageNativeRetPhase,
    current_frame: fn(&P) -> SourceCrtNativeRetFrame,
}
impl<D, P, S, T> SourceCrtStorageCallbackNativeRet<D, P, S, T> {
    pub fn bind_completed_free(owner: SourceCrtStorageOwnedFreeCompletion<D, P, S>, target_owner: T,
        bindings: SourceCrtStorageNativeRetBindings<D, P, T>)
        -> Result<Self, SourceCrtStorageNativeRetBindFailure<D, P, S, T>> {
        let before = owner.callback_before_ret;
        let expected = SourceCrtNativeRetExpectation { before: SourceCrtNativeRetFrame {
            actual_rip: before.required_actual_ret_instruction_rip(), actual_rsp: before.source_rsp_before_ret(),
            actual_rax: before.source_rax_before_ret() }, actual_required_target: (bindings.expected_target)(&target_owner) };
        let actual_frame = (bindings.current_frame)(&owner.prefix);
        let actual_delete = (bindings.delete_completion)(&owner.delete_completion);
        let reason = if before.exclusion().is_some() || before.actual_free_return().is_none() {
            Some(SourceCrtStorageNativeRetBindReason::CompletedFreeRequired)
        } else if actual_delete.actual_free_word != owner.allocation.actual_base_word.get()
            || actual_delete.actual_free_word != before.entry().actual_argument_rcx
            || !actual_delete.actual_heap_owner_retained || !actual_delete.actual_disposition_present {
            Some(SourceCrtStorageNativeRetBindReason::DeleteOwnerRequired)
        } else if actual_frame != expected.before {
            Some(SourceCrtStorageNativeRetBindReason::ActualBeforeRetFrameRequired)
        } else { None };
        if let Some(reason) = reason { return Err(SourceCrtStorageNativeRetBindFailure {
            owner, target_owner, reason, expected, actual_frame, actual_delete }); }
        Ok(Self { owner, target_owner, expected, actual_delete,
            phase: SourceCrtStorageNativeRetPhase::ActualReceiptRequired, current_frame: bindings.current_frame })
    }
    pub fn phase(&self) -> SourceCrtStorageNativeRetPhase { self.phase }
    pub fn required_actual_ret(&self) -> SourceCrtNativeRetExpectation { self.expected }
    pub fn request(&self) -> Option<SourceCrtNativeRetExpectation> {
        if self.phase == SourceCrtStorageNativeRetPhase::ActualReceiptRequired { Some(self.expected) } else { None }
    }
    pub fn prefix(&self) -> &P { &self.owner.prefix }
    pub fn accept_actual_ret<N, E>(mut self, actual: N,
        project: fn(&N) -> SourceCrtNativeRetObservation,
        apply: fn(&mut P, &mut S, &mut N) -> Result<(), E>)
        -> Result<SourceCrtStorageCallbackReturned<D, P, S, T, N>,
            SourceCrtNativeReturnFailure<Self, SourceCrtStorageNativeRetError<N, E>>> {
        self.phase = SourceCrtStorageNativeRetPhase::ActualContinuationRequired;
        let current_frame = self.current_frame;
        let (actual, retired) = match accept_source_crt_native_ret_receipt(&mut self.owner, self.expected, actual, project,
            |owner, actual| apply(&mut owner.prefix, &mut owner.services, actual),
            |owner| current_frame(&owner.prefix)) {
            Ok(returned) => returned,
            Err(cause) => return Err(SourceCrtNativeReturnFailure { owner: self, cause }),
        };
        Ok(SourceCrtStorageCallbackReturned { owner: self.owner, target_owner: self.target_owner,
            expected: self.expected, actual_delete: self.actual_delete, actual_retired: retired, actual_return: actual })
    }
    pub fn into_owned_parts(self) -> SourceCrtStorageNativeRetParts<D, P, S, T> {
        SourceCrtStorageNativeRetParts { owner: self.owner, target_owner: self.target_owner,
            expected: self.expected, actual_delete: self.actual_delete, phase: self.phase, current_frame: self.current_frame }
    }
}
impl<D, P, S> SourceCrtStorageOwnedFreeCompletion<D, P, S> {

    pub fn apply_actual_ret_entry_prefix<A, E>(mut self, actual: A,
        apply: fn(SourceCrtNativeRetFrame, &mut P, &mut S, A) -> Result<(), (A, E)>)
        -> Result<Self, SourceCrtNativeReturnFailure<Self, SourceCrtStorageOwnedFreeReceiptError<A, E>>> {
        let before = self.callback_before_ret;
        let expected = SourceCrtNativeRetFrame { actual_rip: before.required_actual_ret_instruction_rip(),
            actual_rsp: before.source_rsp_before_ret(), actual_rax: before.source_rax_before_ret() };
        match apply(expected, &mut self.prefix, &mut self.services, actual) {
            Ok(()) => Ok(self),
            Err((actual, cause)) => Err(SourceCrtNativeReturnFailure { owner: self,
                cause: SourceCrtStorageOwnedFreeReceiptError::Rejected { actual, cause } }),
        }
    }
}
