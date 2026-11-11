use super::crt_cached_resolver::SourceCrtCachedResolverCompletion;
use super::crt_native_thunk::SourceCrtResolver59bd70Request;
use super::crt_storage_native_return::{SourceCrtNativeRetFrame, SourceCrtNativeRetExpectation,
    SourceCrtNativeRetObservation, SourceCrtNativeRetired, SourceCrtNativeRetPhase,
    SourceCrtNativeRetError, SourceCrtNativeReturnFailure, accept_source_crt_native_ret_receipt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceCrtCachedResolverReturnFrame {
    pub actual_module_base: u64,
    pub actual_entry_rip: u64,
    pub actual_call: SourceCrtResolver59bd70Request,

    pub actual_return_slot_address: u64,
}

pub struct SourceCrtCachedNativeRetBindings<O, T> {
    pub source_completion: fn(&O) -> SourceCrtCachedResolverCompletion,
    pub actual_call_frame: fn(&O) -> SourceCrtCachedResolverReturnFrame,
    pub current_frame: fn(&O) -> SourceCrtNativeRetFrame,
    pub expected_target: fn(&T) -> u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceCrtCachedNativeRetBindReason { ActualCallBindingRequired, ActualEpilogueRequired }
#[must_use = "retain original resolver/module/cache/FP/frame resources and actual target record"]
pub struct SourceCrtCachedNativeRetBindFailure<O, T> {
    pub owner: O,
    pub target_owner: T,
    pub reason: SourceCrtCachedNativeRetBindReason,
    pub source: SourceCrtCachedResolverCompletion,
    pub actual_call_frame: SourceCrtCachedResolverReturnFrame,
    pub expected: SourceCrtNativeRetExpectation,
    pub actual_frame: SourceCrtNativeRetFrame,
}
#[must_use = "retain SAME resolver/module/cache/native/FP/frame resources and actual RET"]
pub struct SourceCrtCachedResolverReturned<O, T, N> {
    pub owner: O,
    pub target_owner: T,
    pub source: SourceCrtCachedResolverCompletion,
    pub actual_call_frame: SourceCrtCachedResolverReturnFrame,
    pub expected: SourceCrtNativeRetExpectation,
    pub actual_retired: SourceCrtNativeRetired,
    pub actual_return: N,
}
pub struct SourceCrtCachedNativeRetParts<O, T> {
    pub owner: O,
    pub target_owner: T,
    pub source: SourceCrtCachedResolverCompletion,
    pub actual_call_frame: SourceCrtCachedResolverReturnFrame,
    pub expected: SourceCrtNativeRetExpectation,
    pub phase: SourceCrtNativeRetPhase,
    pub current_frame: fn(&O) -> SourceCrtNativeRetFrame,
}

#[must_use = "retain SAME cached-resolver/native-resource return continuation"]
pub struct SourceCrtCachedResolverNativeRet<O, T> {
    owner: O,
    target_owner: T,
    source: SourceCrtCachedResolverCompletion,
    actual_call_frame: SourceCrtCachedResolverReturnFrame,
    expected: SourceCrtNativeRetExpectation,
    phase: SourceCrtNativeRetPhase,
    current_frame: fn(&O) -> SourceCrtNativeRetFrame,
}
impl<O, T> SourceCrtCachedResolverNativeRet<O, T> {
    pub fn bind_completed_resolver(owner: O, target_owner: T, bindings: SourceCrtCachedNativeRetBindings<O, T>)
        -> Result<Self, SourceCrtCachedNativeRetBindFailure<O, T>> {
        let source = (bindings.source_completion)(&owner);
        let actual_call_frame = (bindings.actual_call_frame)(&owner);
        let actual_frame = (bindings.current_frame)(&owner);
        let expected = SourceCrtNativeRetExpectation { before: SourceCrtNativeRetFrame {
            actual_rip: actual_call_frame.actual_module_base.wrapping_add(0x59bebd),
            actual_rsp: actual_call_frame.actual_return_slot_address, actual_rax: source.return_rax() },
            actual_required_target: (bindings.expected_target)(&target_owner) };
        let expected_cache = actual_call_frame.actual_module_base.wrapping_add(
            0x861cd8 + u64::from(actual_call_frame.actual_call.selector_ecx) * 8);
        let reason = if source.call() != actual_call_frame.actual_call
            || actual_call_frame.actual_entry_rip != actual_call_frame.actual_module_base.wrapping_add(0x59bd70)
            || actual_call_frame.actual_call.actual_function_cache_address != expected_cache {
            Some(SourceCrtCachedNativeRetBindReason::ActualCallBindingRequired)
        } else if actual_frame != expected.before {
            Some(SourceCrtCachedNativeRetBindReason::ActualEpilogueRequired)
        } else { None };
        if let Some(reason) = reason { return Err(SourceCrtCachedNativeRetBindFailure {
            owner, target_owner, reason, source, actual_call_frame, expected, actual_frame }); }
        Ok(Self { owner, target_owner, source, actual_call_frame, expected,
            phase: SourceCrtNativeRetPhase::ActualReceiptRequired, current_frame: bindings.current_frame })
    }
    pub fn phase(&self) -> SourceCrtNativeRetPhase { self.phase }
    pub fn required_actual_ret(&self) -> SourceCrtNativeRetExpectation { self.expected }
    pub fn request(&self) -> Option<SourceCrtNativeRetExpectation> {
        if self.phase == SourceCrtNativeRetPhase::ActualReceiptRequired { Some(self.expected) } else { None }
    }
    pub fn owner(&self) -> &O { &self.owner }
    pub fn accept_actual_ret<N, E>(mut self, actual: N, project: fn(&N) -> SourceCrtNativeRetObservation,
        apply: fn(&mut O, &mut N) -> Result<(), E>)
        -> Result<SourceCrtCachedResolverReturned<O, T, N>, SourceCrtNativeReturnFailure<Self, SourceCrtNativeRetError<N, E>>> {
        self.phase = SourceCrtNativeRetPhase::ActualContinuationRequired;
        let (actual, retired) = match accept_source_crt_native_ret_receipt(&mut self.owner, self.expected,
            actual, project, apply, self.current_frame) {
            Ok(returned) => returned,
            Err(cause) => return Err(SourceCrtNativeReturnFailure { owner: self, cause }),
        };
        Ok(SourceCrtCachedResolverReturned { owner: self.owner, target_owner: self.target_owner,
            source: self.source, actual_call_frame: self.actual_call_frame, expected: self.expected,
            actual_retired: retired, actual_return: actual })
    }
    pub fn into_owned_parts(self) -> SourceCrtCachedNativeRetParts<O, T> {
        SourceCrtCachedNativeRetParts { owner: self.owner, target_owner: self.target_owner,
            source: self.source, actual_call_frame: self.actual_call_frame, expected: self.expected,
            phase: self.phase, current_frame: self.current_frame }
    }
}
