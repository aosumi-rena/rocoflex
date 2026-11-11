use super::crt_query_initialization::{SourceCrtQueryInitialization58df48,
    SourceCrtQueryInitializationBody, SourceCrtQueryInitializationLastCall,
    SourceCrtQueryInitializationBodyFacts, SourceCrtQueryInitializationReceipt,
    SourceCrtQueryInitializationProgress, SourceCrtQueryInitializationError};
use super::crt_storage_native_return::SourceCrtNativeRetFrame;

pub struct SourceCrtQueryInitializationBodyBindings<D, P, S> {
    pub actual_body: fn(&D) -> SourceCrtQueryInitializationBody,
    pub required_next_pc: fn(&D) -> u32,
    pub last_actual_call_return: fn(&D) -> Option<SourceCrtQueryInitializationLastCall>,
    pub actual_frame: fn(&D, &P, &S) -> SourceCrtNativeRetFrame,
    pub actual_rbp: fn(&D, &P, &S) -> u64,
}
#[must_use = "retain SAME private body/provider/storage/two child caps/full frame/FP/resources"]
pub struct SourceCrtQueryInitializationBodyCapsule<D, P, S> {
    pub body: D, pub prefix: P, pub services: S,
    pub bindings: SourceCrtQueryInitializationBodyBindings<D, P, S>,
}
#[must_use = "retain accepted body capsule through required actual epilogue and RET"]
pub struct SourceCrtQueryInitializationBodyAccepted<D, P, S> {
    pub actual: SourceCrtQueryInitializationBodyCapsule<D, P, S>,
    pub source: SourceCrtQueryInitializationProgress,
}
#[must_use = "retain entire rejected body with stopped original initializer"]
pub struct SourceCrtQueryInitializationBodyError<D, P, S> {
    pub actual: SourceCrtQueryInitializationBodyCapsule<D, P, S>,
    pub pending: SourceCrtQueryInitializationProgress,
    pub observed_body: SourceCrtQueryInitializationBody,
    pub required_next_pc: u32,
    pub last_actual_call: Option<SourceCrtQueryInitializationLastCall>,
    pub actual_frame: SourceCrtNativeRetFrame, pub actual_rbp: u64,
    pub source: Option<SourceCrtQueryInitializationError>,
}
impl SourceCrtQueryInitialization58df48 {
    pub fn accept_owned_body<D, P, S>(&mut self, actual: SourceCrtQueryInitializationBodyCapsule<D, P, S>)
        -> Result<SourceCrtQueryInitializationBodyAccepted<D, P, S>, SourceCrtQueryInitializationBodyError<D, P, S>> {
        self.stop();
        let pending = self.progress();
        let observed_body = (actual.bindings.actual_body)(&actual.body);
        let required_next_pc = (actual.bindings.required_next_pc)(&actual.body);
        let last_actual_call = (actual.bindings.last_actual_call_return)(&actual.body);
        let actual_frame = (actual.bindings.actual_frame)(&actual.body, &actual.prefix, &actual.services);
        let actual_rbp = (actual.bindings.actual_rbp)(&actual.body, &actual.prefix, &actual.services);
        let Some(last_actual_call_value) = last_actual_call else {
            return Err(SourceCrtQueryInitializationBodyError { actual, pending, observed_body, required_next_pc,
                last_actual_call, actual_frame, actual_rbp, source: None });
        };
        let receipt = SourceCrtQueryInitializationReceipt::BorrowedBody {
            facts: SourceCrtQueryInitializationBodyFacts { actual: observed_body, required_next_pc,
                last_actual_call: last_actual_call_value }, actual_frame, actual_rbp };
        match self.accept_actual_completion(receipt) {
            Ok(source) => Ok(SourceCrtQueryInitializationBodyAccepted { actual, source }),
            Err(source) => Err(SourceCrtQueryInitializationBodyError { actual, pending, observed_body, required_next_pc,
                last_actual_call, actual_frame, actual_rbp, source: Some(source) }),
        }
    }
}
