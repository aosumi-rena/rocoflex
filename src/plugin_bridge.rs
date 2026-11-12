use std::{ffi::{c_char,CStr},path::Path,ptr,sync::{Mutex,TryLockError,
    atomic::{AtomicBool,Ordering}},panic::{catch_unwind,AssertUnwindSafe}};
use crate::{Result,current_condition::{ConditionAbsence,ConditionBus,ConditionReader,ConditionTarget},
    prototype_controls::{Parameters,Snapshot},morph_processor::{PluginProcessor,LatencyPreparation,PLUGIN_LATENCY_SAMPLES,input_gain::InputGain}};

const OK:i32=0;
const NOT_READY:i32=1;
const BUSY:i32=2;
const INVALID:i32=-1;
const FAILED:i32=-2;
const PANIC:i32=-3;

#[cfg(not(target_arch="x86_64"))]
compile_error!("The VST3 bridge currently supports the Windows x86-64 floating-point contract.");

struct HostFpEnvironment { saved:u32 }
impl HostFpEnvironment {
    fn enter()->Self {

        unsafe {
            let saved=std::arch::x86_64::_mm_getcsr();
            std::arch::x86_64::_mm_setcsr(0x1f80);
            Self {saved}
        }
    }
}
impl Drop for HostFpEnvironment {
    fn drop(&mut self) {

        unsafe {std::arch::x86_64::_mm_setcsr(self.saved);}
    }
}

#[repr(C)]
#[derive(Clone,Copy)]
pub struct RocoCondition {
    pub mode:u32,
    pub pregain_db:f32,
    pub pitch_semitones:f32,
    pub tone:[f32;3],
    pub canvas:[f32;128],
}
const _: [();536]=[();std::mem::size_of::<RocoCondition>()];
const _: [();4]=[();std::mem::align_of::<RocoCondition>()];
impl Default for RocoCondition {
    fn default()->Self {
        Self {mode:0,pregain_db:0.,pitch_semitones:0.,tone:[0.;3],canvas:[0.;128]}
    }
}

#[repr(C)]
#[derive(Clone,Copy,Debug,Default,PartialEq,Eq)]
pub struct RocoPrepareInfo {pub delay_samples:u32,pub policy:u32}
const _: [();8]=[();std::mem::size_of::<RocoPrepareInfo>()];
const _: [();4]=[();std::mem::align_of::<RocoPrepareInfo>()];
const _: [();0]=[();std::mem::offset_of!(RocoPrepareInfo,delay_samples)];
const _: [();4]=[();std::mem::offset_of!(RocoPrepareInfo,policy)];
impl RocoPrepareInfo {
    fn valid(self)->bool {
        self.delay_samples!=0 && (self.policy==2 || (self.policy==1 && self.delay_samples==PLUGIN_LATENCY_SAMPLES))
    }
}

unsafe fn finish_prepare_output(out:*mut RocoPrepareInfo,code:i32,completed:Option<RocoPrepareInfo>)->i32 {
    if out.is_null() {return if code==OK {INVALID} else {code};}
    let result=match (code,completed) {
        (OK,Some(info)) if info.valid()=>{unsafe {out.write(info);}return OK;},
        (OK,_)=>FAILED,
        (other,_)=>other,
    };
    unsafe {out.write(RocoPrepareInfo::default());}
    result
}

fn prepare_audio_buffers(frames:usize)->Result<(Vec<f32>,Vec<f32>)> {
    if !(1..=65_536).contains(&frames) {return Err("Invalid prepared audio scratch extent".into());}
    let mut input=Vec::new();input.try_reserve_exact(frames).map_err(|e|e.to_string())?;input.resize(frames,0.);
    let mut output=Vec::new();output.try_reserve_exact(frames).map_err(|e|e.to_string())?;output.resize(frames,0.);
    Ok((input,output))
}
#[repr(C)]
#[derive(Clone,Copy,Default)]
pub struct RocoPitch {pub word:u64,pub ordinal:u64}
const _: [();16]=[();std::mem::size_of::<RocoPitch>()];
const _: [();8]=[();std::mem::align_of::<RocoPitch>()];

struct Ready {
    processor:PluginProcessor,
    input_channel:usize,
    input:Vec<f32>,
    output:Vec<f32>,
}

struct Runtime {ready:Option<Ready>,gain:InputGain}

pub struct RocoPlugin {
    bus:ConditionBus,
    #[cfg(feature="desktop")]
    editor_session:std::sync::Arc<crate::editor_session::Store<crate::desktop::CompletedEditorSession>>,
    #[cfg(feature="desktop")]
    multithread_edits:std::sync::Arc<std::sync::atomic::AtomicU8>,
    condition:Mutex<ConditionReader>,
    ready:Mutex<Runtime>,
    error:Mutex<String>,
    panicked:AtomicBool,

}

struct Failure(i32,String);
type CallResult=std::result::Result<(),Failure>;
fn invalid(message:&str)->Failure {Failure(INVALID,message.into())}
fn failed(message:String)->Failure {Failure(FAILED,message)}

impl RocoPlugin {
    fn error(&self,message:String) {
        *self.error.lock().unwrap_or_else(std::sync::PoisonError::into_inner)=message;
    }
    fn call(&self,body:impl FnOnce()->CallResult)->i32 {
        match catch_unwind(AssertUnwindSafe(body)) {
            Ok(Ok(()))=>OK,
            Ok(Err(Failure(code,message)))=>{self.error(message);code},
            Err(_)=>{
                self.panicked.store(true,Ordering::Release);
                self.error("Plugin operation panicked; prepare again before processing.".into());
                PANIC
            }
        }
    }
}

pub(crate) unsafe fn condition_bus(handle:*mut RocoPlugin)->Result<ConditionBus> {

    unsafe {handle.as_ref()}.map(|plugin|plugin.bus.clone())
        .ok_or_else(||"Missing plugin control handle.".into())
}

#[cfg(feature="desktop")]
pub(crate) unsafe fn editor_session_store(handle:*mut RocoPlugin)
    ->Result<std::sync::Arc<crate::editor_session::Store<crate::desktop::CompletedEditorSession>>>
{
    unsafe {handle.as_ref()}.map(|plugin|plugin.editor_session.clone())
        .ok_or_else(||"Missing plugin control handle.".into())
}

#[cfg(feature="desktop")]
pub(crate) unsafe fn multithread_mailbox(handle:*mut RocoPlugin)
    ->Result<std::sync::Arc<std::sync::atomic::AtomicU8>>
{
    unsafe {handle.as_ref()}.map(|plugin|plugin.multithread_edits.clone())
        .ok_or_else(||"Missing plugin control handle.".into())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_plugin_invalidate_editor_session(handle:*mut RocoPlugin)->i32 {
    let Some(plugin)=(unsafe {handle.as_ref()}) else {return INVALID;};
    plugin.call(|| {
        #[cfg(feature="desktop")]
        plugin.editor_session.invalidate().map_err(failed)?;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn roco_plugin_create()->*mut RocoPlugin {
    catch_unwind(||{
        let bus=ConditionBus::new();
        Box::into_raw(Box::new(RocoPlugin {condition:Mutex::new(bus.reader()),bus,
            #[cfg(feature="desktop")]
            editor_session:std::sync::Arc::new(crate::editor_session::Store::default()),
            #[cfg(feature="desktop")]
            multithread_edits:std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0)),
            ready:Mutex::new(Runtime {ready:None,gain:InputGain::new()}),error:Mutex::new(String::new()),panicked:AtomicBool::new(false)}))
    }).unwrap_or(ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn roco_plugin_read_multithread_preference()->u32 {
    catch_unwind(|| {
        #[cfg(feature="desktop")]
        { u32::from(crate::desktop::read_multithread_preference()) }
        #[cfg(not(feature="desktop"))]
        { 1 }
    }).unwrap_or(1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_plugin_try_read_multithread_preference(out:*mut u32)->i32 {
    if out.is_null() {return INVALID;}
    match catch_unwind(AssertUnwindSafe(|| {
        #[cfg(feature="desktop")]
        { crate::desktop::try_read_multithread_preference().map(u32::from).map_err(|_| ()) }
        #[cfg(not(feature="desktop"))]
        { Ok::<u32,()>(1) }
    })) {
        Ok(Ok(value))=>{unsafe {out.write(value);}OK},
        Ok(Err(()))=>FAILED,
        Err(_)=>PANIC,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_plugin_take_multithread_edit(handle:*const RocoPlugin,out:*mut u32)->i32 {
    let Some(plugin)=(unsafe {handle.as_ref()}) else {return INVALID;};
    if out.is_null() {return INVALID;}
    #[cfg(feature="desktop")]
    {
        match plugin.multithread_edits.swap(0,Ordering::AcqRel) {
            0=>0,
            1=>{unsafe {out.write(0);}1},
            2=>{unsafe {out.write(1);}1},
            _=>INVALID,
        }
    }
    #[cfg(not(feature="desktop"))]
    {let _=plugin;0}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_plugin_prepare(handle:*mut RocoPlugin,
    analysis:*const c_char,userdata:*const c_char,sample_rate:u32,
    max_block_frames:u32,input_channel:u32,profile:u32,latency_policy:u32,
    multithread:u32,out:*mut RocoPrepareInfo)->i32
{

    if !out.is_null() {unsafe {out.write(RocoPrepareInfo::default());}}

    let Some(plugin)=(unsafe {handle.as_ref()}) else {return INVALID;};
    let _fp=HostFpEnvironment::enter();
    let mut completed=None;
    let code=plugin.call(||{

        {
            let mut slot=plugin.ready.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            slot.ready=None;
            plugin.ready.clear_poison();
        }
        if out.is_null() {return Err(invalid("Missing prepared latency output."));}
        let profile=crate::prepare_quality::PrepareProfile::try_from(profile)
            .map_err(|error|Failure(NOT_READY,error))?;
        let latency_policy=LatencyPreparation::try_from(latency_policy).map_err(|e|Failure(INVALID,e))?;
        if multithread>1 {return Err(invalid("Invalid per-instance multithread preference."));}
        if analysis.is_null() || userdata.is_null() {return Err(invalid("Missing model path."));}
        if input_channel>1 {return Err(invalid("Input channel must be left or right."));}

        let analysis=unsafe {CStr::from_ptr(analysis)}.to_str().map_err(|_|invalid("Analysis path is not UTF-8."))?;
        let userdata=unsafe {CStr::from_ptr(userdata)}.to_str().map_err(|_|invalid("Userdata path is not UTF-8."))?;
        if analysis.is_empty() || userdata.is_empty() {return Err(invalid("Select both model files."));}
        let processor=PluginProcessor::with_instance_policy(Path::new(analysis),Path::new(userdata),&plugin.bus,
            sample_rate,max_block_frames as usize,profile,latency_policy,multithread!=0).map_err(failed)?;
        let latency=processor.prepared_latency();
        let info=RocoPrepareInfo {delay_samples:latency.delay_samples(),policy:latency.policy()};
        let (input,output)=prepare_audio_buffers(max_block_frames as usize).map_err(failed)?;
        let ready=Ready {processor,input_channel:input_channel as usize,input,output};
        let mut slot=plugin.ready.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        slot.ready=Some(ready);
        plugin.ready.clear_poison();
        plugin.panicked.store(false,Ordering::Release);
        plugin.error(String::new());
        completed=Some(info);
        Ok(())
    });
    unsafe {finish_prepare_output(out,code,completed)}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_plugin_set_condition(handle:*mut RocoPlugin,value:*const RocoCondition)->i32 {

    let Some(plugin)=(unsafe {handle.as_ref()}) else {return INVALID;};
    plugin.call(||{
        let value=unsafe {value.as_ref()}.ok_or_else(||invalid("Missing condition value."))?;
        let parameters=Parameters {pregain_db:value.pregain_db,
            pitch_semitones:value.pitch_semitones,tone_db:0.};
        match value.mode {
            0=>{plugin.bus.publish_empty().map_err(failed)?;},
            1=>{plugin.bus.publish_canvas(Snapshot {embedding:value.canvas,targets:Vec::new(),parameters}).map_err(failed)?;},
            2=>{plugin.bus.publish_tone_preview(value.tone,parameters).map_err(failed)?;},
            3=>{plugin.bus.publish_unavailable().map_err(failed)?;},
            _=>return Err(invalid("Unknown plugin condition mode.")),
        }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_plugin_get_condition(handle:*mut RocoPlugin,
    value:*mut RocoCondition,revision:*mut u64)->i32
{

    let Some(plugin)=(unsafe {handle.as_ref()}) else {return INVALID;};
    plugin.call(||{
        if value.is_null() || revision.is_null() {return Err(invalid("Missing condition output."));}
        let mut reader=plugin.condition.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut result=RocoCondition::default();
        if let Some(current)=reader.poll() {
            result.pregain_db=current.pregain_db;result.pitch_semitones=current.pitch_semitones;
            match &current.target {
                ConditionTarget::Canvas128 {embedding,..}=>{result.mode=1;result.canvas=*embedding;},
                ConditionTarget::TonePreview {controls}=>{result.mode=2;result.tone=*controls;},
            }
        } else if reader.absence()==Some(ConditionAbsence::Unavailable) {
            result.mode=3;
        }

        unsafe {value.write(result);revision.write(reader.revision());}
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_plugin_input_gain(handle:*const RocoPlugin,value:*mut f32)->i32 {
    let Some(plugin)=(unsafe {handle.as_ref()}) else {return INVALID;};
    if value.is_null() {return INVALID;}
    catch_unwind(AssertUnwindSafe(||match plugin.ready.try_lock() {
        Ok(runtime)=>{unsafe {value.write(runtime.gain.value());} OK},
        Err(TryLockError::WouldBlock)=>BUSY,
        Err(TryLockError::Poisoned(_))=>FAILED,
    })).unwrap_or(PANIC)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_plugin_process(handle:*mut RocoPlugin,
    in_left:*const f32,in_right:*const f32,out_left:*mut f32,out_right:*mut f32,
    frames:u32,input_channels:u32,output_channels:u32,source_epoch:u64,source_channel:u32,source_bus:u64,pitch_out:*mut RocoPitch)->i32
{
    if !pitch_out.is_null() {unsafe {pitch_out.write(RocoPitch::default());}}
    let collect_pitch=!pitch_out.is_null() && source_epoch!=0 && source_channel<=1 && source_bus>>1!=0 && source_bus&1!=0;
    let Some(plugin)=(unsafe {handle.as_ref()}) else {return INVALID;};
    let _fp=HostFpEnvironment::enter();
    plugin.call(||{
        let count=frames as usize;
        if count==0 {return Ok(());}
        if count>65_536 || !(1..=2).contains(&output_channels) || out_left.is_null()
            || (output_channels==2 && out_right.is_null()) {
            return Err(invalid("Invalid plugin output buffer dimensions."));
        }

        let silence=||unsafe {
            ptr::write_bytes(out_left,0,count);
            if output_channels==2 {ptr::write_bytes(out_right,0,count);}
        };
        if plugin.panicked.load(Ordering::Acquire) {
            silence();return Err(failed("Plugin must be prepared after a panic.".into()));
        }
        let mut slot=match plugin.ready.try_lock() {
            Ok(slot)=>slot,
            Err(TryLockError::WouldBlock)=>{silence();return Err(Failure(BUSY,"Plugin preparation is in progress.".into()));},
            Err(TryLockError::Poisoned(_))=>{silence();return Err(failed("Plugin processing state is poisoned; prepare again.".into()));},
        };
        let Runtime {ready,gain}=&mut *slot;
        let Some(ready)=ready.as_mut() else {
            silence();return Err(Failure(NOT_READY,"Load both model files before processing.".into()));
        };
        if count>ready.input.len() || !(1..=2).contains(&input_channels)
            || ready.input_channel>=input_channels as usize {
            silence();return Err(invalid("Audio block or selected input channel differs from preparation."));
        }
        let source=if ready.input_channel==0 {in_left} else {in_right};
        if source.is_null() {silence();return Err(invalid("Missing selected input buffer."));}

        ready.input[..count].copy_from_slice(unsafe {std::slice::from_raw_parts(source,count)});
        silence();
        if !ready.processor.has_condition() {
            return Err(Failure(NOT_READY,"Select a canvas voice or Tone preview before processing.".into()));
        }
        if collect_pitch {ready.processor.set_pitch_source(source_epoch,source_channel,source_bus);}
        else {ready.processor.set_pitch_source(0,0,0);}
        ready.processor.process(&ready.input[..count],&mut ready.output[..count],gain).map_err(failed)?;
        if collect_pitch {
            let pitch=ready.processor.pitch_sample();
            unsafe {pitch_out.write(RocoPitch{word:pitch.word,ordinal:pitch.ordinal});}
        }

        unsafe {
            ptr::copy_nonoverlapping(ready.output.as_ptr(),out_left,count);
            if output_channels==2 {ptr::copy_nonoverlapping(ready.output.as_ptr(),out_right,count);}
        }
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_plugin_latency_samples(handle:*const RocoPlugin)->u32 {
    if handle.is_null() {0} else {PLUGIN_LATENCY_SAMPLES}
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_plugin_copy_error(handle:*const RocoPlugin,buffer:*mut c_char,capacity:usize)->usize {
    catch_unwind(AssertUnwindSafe(||{

        let Some(plugin)=(unsafe {handle.as_ref()}) else {return 0;};
        let error=plugin.error.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if !buffer.is_null() && capacity>0 {
            let count=error.len().min(capacity-1);
            unsafe {ptr::copy_nonoverlapping(error.as_ptr(),buffer.cast::<u8>(),count);buffer.add(count).write(0);}
        }
        error.len()+1
    })).unwrap_or(0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn roco_plugin_destroy(handle:*mut RocoPlugin) {
    if handle.is_null() {return;}

    let _=catch_unwind(AssertUnwindSafe(||unsafe {drop(Box::from_raw(handle));}));
}

