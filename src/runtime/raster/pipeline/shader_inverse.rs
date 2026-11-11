use super::{Next, Progress as PrefixProgress, Required, Services as PrefixServices};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Site { At27a60d, At27a6aa }
impl Site {
    pub fn call_rva(self) -> u32 { match self { Self::At27a60d=>0x27a60d,Self::At27a6aa=>0x27a6aa } }

    pub fn return_rva(self) -> u32 { match self { Self::At27a60d=>0x27a612,Self::At27a6aa=>0x27a6af } }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Arguments { pub rcx_transform: u64, pub rdx_output: u64 }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Owners<I> {
    pub invocation: I, pub caller_context: I, pub frame: I, pub services: I,
    pub source_module: I, pub work_stack: I, pub descriptor_resources: I, pub fp: I,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Target<I> {
    pub owners: Owners<I>, pub site: Site, pub arguments: Arguments,
    pub callee_module: I, pub callee_context: I, pub original_transform_backing: I,
    pub native_output_backing: I, pub receipt_owner: I,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Observation {

    pub raw_rax: u64, pub mxcsr: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReturnKey<I> {
    pub target: Target<I>, pub callee_frame: I, pub receipt_identity: I, pub observed: Observation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectsKey<I> {
    pub target: Target<I>, pub callee_frame: I, pub effects_identity: I,
}
#[must_use = "actual whole normal-return packet stays with its original frame"]
pub struct Returned<I,P> { pub key: ReturnKey<I>, pub packet: P }
#[must_use = "native output effects are separate from numerical inverse/normal-return evidence"]
pub struct NativeEffects<I,P> { pub key: EffectsKey<I>, pub packet: P }
#[must_use]
pub struct NormalReturn<I,R,O> {
    pub observed: Observation, pub returned: Option<Returned<I,R>>, pub output: Option<NativeEffects<I,O>>,
}
#[must_use = "retain ALL partial call/output receipts and original provider error"]
pub struct CallFailure<I,R,O,E> {
    pub error: E, pub actual_normal_return: Option<Observation>,
    pub returned: Option<Returned<I,R>>, pub output: Option<NativeEffects<I,O>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Retained<I> { pub returned: ReturnKey<I>, pub output: EffectsKey<I>, pub frame_owner: I }
#[must_use]
pub struct RetentionFailure<I,R,O,E> {
    pub error: E, pub returned: Option<Returned<I,R>>, pub output: Option<NativeEffects<I,O>>,
    pub retained: Option<Retained<I>>,
}
pub trait Services: PrefixServices {
    type Identity: Copy + Eq;
    type ReturnPacket;
    type OutputPacket;

    fn inverse_owners(&mut self, frame: &mut Self::Frame, site: Site)
        -> Result<Owners<Self::Identity>,Self::Error>;

    fn inverse_arguments(&mut self, frame: &mut Self::Frame) -> Result<Arguments,Self::Error>;
    fn bind_inverse_211800(&mut self, frame: &mut Self::Frame, owners: &Owners<Self::Identity>,
        site: Site, arguments: Arguments) -> Result<Target<Self::Identity>,Self::Error>;

    fn actual_inverse_211800(&mut self, frame: &mut Self::Frame, target: &Target<Self::Identity>)
        -> Result<NormalReturn<Self::Identity,Self::ReturnPacket,Self::OutputPacket>,
            CallFailure<Self::Identity,Self::ReturnPacket,Self::OutputPacket,Self::Error>>;
    fn validate_inverse_return(&mut self, frame: &mut Self::Frame,
        returned: &Returned<Self::Identity,Self::ReturnPacket>) -> Result<(),Self::Error>;

    fn validate_inverse_output(&mut self, frame: &mut Self::Frame,
        output: &NativeEffects<Self::Identity,Self::OutputPacket>) -> Result<(),Self::Error>;

    fn validate_inverse_continuation(&mut self, frame: &mut Self::Frame,
        target: &Target<Self::Identity>, return_rva: u32) -> Result<(),Self::Error>;

    fn retain_inverse_packets(&mut self, frame: &mut Self::Frame,
        returned: Returned<Self::Identity,Self::ReturnPacket>, output: NativeEffects<Self::Identity,Self::OutputPacket>)
        -> Result<Option<Retained<Self::Identity>>,
            RetentionFailure<Self::Identity,Self::ReturnPacket,Self::OutputPacket,Self::Error>>;
    fn validate_inverse_retention(&mut self, frame: &mut Self::Frame, retained: &Retained<Self::Identity>)
        -> Result<(),Self::Error>;
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase { Entry, Owners, Arguments, Bind, BeforeCall, CallStopped, Returned,
    ValidateReturn, ValidateOutput, ValidateContinuation, Retain, RetentionStopped, ValidateRetention, Complete }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop { WrongPrefixExit, TargetMismatch, MissingReturn, MissingOutput,
    ReturnMismatch, ObservationMismatch, OutputMismatch, CalleeMismatch, MissingRetention, RetentionMismatch }
#[derive(Clone, Copy, Debug)]
pub struct Progress<I> {
    pub phase: Phase, pub attempted_rva: u32, pub site: Option<Site>,
    pub owners: Option<Owners<I>>, pub arguments: Option<Arguments>, pub target: Option<Target<I>>,
    pub observed: Option<Observation>, pub returned: Option<ReturnKey<I>>, pub output: Option<EffectsKey<I>>,
    pub retained: Option<Retained<I>>, pub pending_return_matches: Option<bool>, pub pending_output_matches: Option<bool>,
}
pub enum Cause<E> { Provider(E), Stopped(Stop) }
#[must_use = "partial SAME frame/callee/descriptor/output effects cannot be retried or discarded"]
pub struct Failure<F,I,R,O,E> {
    pub frame: F, pub prefix: PrefixProgress, pub progress: Progress<I>,
    pub returned: Option<Returned<I,R>>, pub output: Option<NativeEffects<I,O>>, pub cause: Cause<E>,
}
#[must_use = "next shader instructions/shared frame are REQUIRED; this is NOT shader completion/RET"]
pub struct AfterReturn<F,I> {
    pub frame: F, pub prefix: PrefixProgress, pub progress: Progress<I>, pub next_rva: u32,
}
pub fn call<S: Services>(host: &mut S, required: Required<S::Frame>)
    -> Result<AfterReturn<S::Frame,S::Identity>,
        Failure<S::Frame,S::Identity,S::ReturnPacket,S::OutputPacket,S::Error>>
{
    let Required { mut frame,progress:prefix,next }=required;
    let mut p=Progress { phase:Phase::Entry,attempted_rva:next.rva(),site:None,owners:None,
        arguments:None,target:None,observed:None,returned:None,output:None,retained:None,
        pending_return_matches:None,pending_output_matches:None };
    let mut pending_return=None; let mut pending_output=None;
    let result=(|| -> Result<u32,Cause<S::Error>> {
        let site=match next { Next::Actual211800CallAt27a60d=>Site::At27a60d,
            Next::Actual211800CallAt27a6aa=>Site::At27a6aa,
            _=>return Err(Cause::Stopped(Stop::WrongPrefixExit)) };
        p.site=Some(site);p.phase=Phase::Owners;
        let owners=host.inverse_owners(&mut frame,site).map_err(Cause::Provider)?;p.owners=Some(owners);
        p.phase=Phase::Arguments;
        let arguments=host.inverse_arguments(&mut frame).map_err(Cause::Provider)?;p.arguments=Some(arguments);
        p.phase=Phase::Bind;
        let target=host.bind_inverse_211800(&mut frame,&owners,site,arguments).map_err(Cause::Provider)?;p.target=Some(target);
        if target.owners!=owners||target.site!=site||target.arguments!=arguments||target.callee_module!=owners.source_module {
            return Err(Cause::Stopped(Stop::TargetMismatch));
        }
        p.phase=Phase::BeforeCall;
        let normal=match host.actual_inverse_211800(&mut frame,&target) {
            Ok(normal)=>normal,
            Err(CallFailure { error,actual_normal_return,returned,output })=> {
                p.phase=Phase::CallStopped;p.observed=actual_normal_return;
                p.returned=returned.as_ref().map(|r|r.key);p.output=output.as_ref().map(|o|o.key);
                pending_return=returned;pending_output=output;return Err(Cause::Provider(error));
            }
        };
        p.phase=Phase::Returned;p.attempted_rva=site.return_rva();p.observed=Some(normal.observed);
        pending_return=normal.returned;pending_output=normal.output;
        p.returned=pending_return.as_ref().map(|r|r.key);p.output=pending_output.as_ref().map(|o|o.key);
        let returned=pending_return.as_ref().ok_or(Cause::Stopped(Stop::MissingReturn))?;
        let output=pending_output.as_ref().ok_or(Cause::Stopped(Stop::MissingOutput))?;
        let return_key=returned.key;let output_key=output.key;
        if return_key.target!=target { return Err(Cause::Stopped(Stop::ReturnMismatch)); }
        if return_key.observed!=normal.observed { return Err(Cause::Stopped(Stop::ObservationMismatch)); }
        if output_key.target!=target { return Err(Cause::Stopped(Stop::OutputMismatch)); }
        if output_key.callee_frame!=return_key.callee_frame { return Err(Cause::Stopped(Stop::CalleeMismatch)); }
        p.phase=Phase::ValidateReturn;host.validate_inverse_return(&mut frame,returned).map_err(Cause::Provider)?;
        p.phase=Phase::ValidateOutput;host.validate_inverse_output(&mut frame,output).map_err(Cause::Provider)?;
        p.phase=Phase::ValidateContinuation;
        host.validate_inverse_continuation(&mut frame,&target,site.return_rva()).map_err(Cause::Provider)?;
        p.phase=Phase::Retain;

        let (returned,output)=match (pending_return.take(),pending_output.take()) {
            (Some(returned),Some(output))=>(returned,output),
            (returned,output)=> { pending_return=returned;pending_output=output;
                return Err(Cause::Stopped(Stop::MissingRetention)); }
        };
        let retained=match host.retain_inverse_packets(&mut frame,returned,output) {
            Ok(Some(retained))=>retained,
            Ok(None)=>return Err(Cause::Stopped(Stop::MissingRetention)),
            Err(RetentionFailure { error,returned,output,retained })=> {
                p.phase=Phase::RetentionStopped;p.retained=retained;
                p.pending_return_matches=returned.as_ref().map(|r|r.key==return_key);
                p.pending_output_matches=output.as_ref().map(|o|o.key==output_key);
                pending_return=returned;pending_output=output;return Err(Cause::Provider(error));
            }
        };
        p.retained=Some(retained);
        if retained.returned!=return_key||retained.output!=output_key||retained.frame_owner!=owners.frame {
            return Err(Cause::Stopped(Stop::RetentionMismatch));
        }
        p.phase=Phase::ValidateRetention;host.validate_inverse_retention(&mut frame,&retained).map_err(Cause::Provider)?;
        p.phase=Phase::Complete;Ok(site.return_rva())

    })();
    match result { Ok(next_rva)=>Ok(AfterReturn { frame,prefix,progress:p,next_rva }),
        Err(cause)=>Err(Failure { frame,prefix,progress:p,returned:pending_return,output:pending_output,cause }) }
}
