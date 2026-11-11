use std::{panic::{catch_unwind,AssertUnwindSafe},sync::{Arc,atomic::{AtomicBool,Ordering},
    mpsc::{self,Receiver,SyncSender,TrySendError}},thread::{self,JoinHandle},time::Duration};
use crate::Result;
use super::super::vocoder::{VocoderStream,VocoderStartupProgress};

const HOP:usize=240;
const CONDITION:usize=72;
const WAIT:Duration=Duration::from_millis(100);

trait HopProcessor:Send+'static {
    type Progress:Copy+Send+'static;
    fn run(&mut self,signal:&[f32;HOP],condition:&[f32;CONDITION],out:&mut Vec<f32>)->Result<()>;
    fn progress(&self)->Self::Progress;
}
impl HopProcessor for VocoderStream {
    type Progress=VocoderStartupProgress;
    fn run(&mut self,signal:&[f32;HOP],condition:&[f32;CONDITION],out:&mut Vec<f32>)->Result<()> {
        self.process(signal,condition,out)
    }
    fn progress(&self)->Self::Progress {self.startup_progress()}
}

struct Job {sequence:u64,signal:[f32;HOP],condition:[f32;CONDITION]}
struct Completion<P> {
    sequence:u64,samples:[f32;HOP],len:usize,progress:Option<P>,
    error:Option<String>,
}
struct PersistentWorker<P:Copy+Send+'static> {
    requests:Option<SyncSender<Job>>,results:Receiver<Completion<P>>,
    thread:Option<JoinHandle<()>>,cancel:Arc<AtomicBool>,
    pending:Option<u64>,next:u64,progress:P,current:bool,
    failed:bool,
}
impl<P:Copy+Send+'static> PersistentWorker<P> {
    fn spawn<T:HopProcessor<Progress=P>>(mut processor:T)->Result<Self> {
        let progress=processor.progress();
        let (requests,jobs)=mpsc::sync_channel::<Job>(1);
        let (completed,results)=mpsc::sync_channel(1);
        let (started,ready)=mpsc::sync_channel(1);
        let cancel=Arc::new(AtomicBool::new(false));
        let worker_cancel=cancel.clone();
        let thread=thread::Builder::new().name("Rocoflex vocoder".into()).spawn(move || {
            let mut scratch=Vec::with_capacity(HOP);
            if started.send(()).is_err() {return;}
            while let Ok(job)=jobs.recv() {
                if worker_cancel.load(Ordering::Acquire) {break;}
                let sequence=job.sequence;
                let outcome=catch_unwind(AssertUnwindSafe(|| {

                    let _fp=WorkerFp::enter();
                    scratch.clear();
                    let result=processor.run(&job.signal,&job.condition,&mut scratch);
                    let progress=processor.progress();
                    let error=match result {
                        Err(error)=>Some(error),
                        Ok(()) if scratch.len()>HOP=>Some("Vocoder worker output exceeds one hop".into()),
                        Ok(()) if scratch.iter().any(|x|!x.is_finite())=>Some("Nonfinite vocoder worker output".into()),
                        Ok(())=>None,
                    };
                    let mut samples=[0.;HOP];
                    let len=if error.is_none() {scratch.len()} else {0};
                    samples[..len].copy_from_slice(&scratch[..len]);
                    Completion {sequence,samples,len,progress:Some(progress),error}
                }));
                let packet=outcome.unwrap_or_else(|_|Completion {sequence,samples:[0.;HOP],len:0,
                    progress:None,error:Some("Vocoder worker panicked; prepare again".into())});
                let stop=packet.error.is_some();

                if completed.try_send(packet).is_err() || stop {break;}
            }
        }).map_err(|e|format!("Cannot start vocoder worker: {e}"))?;
        if ready.recv_timeout(WAIT).is_err() {
            cancel.store(true,Ordering::Release);
            drop(requests);

            let _=thread.join();
            return Err("Vocoder worker startup did not complete".into());
        }
        Ok(Self {requests:Some(requests),results,thread:Some(thread),cancel,
            pending:None,next:0,progress,current:true,failed:false})
    }
    fn submit(&mut self,signal:[f32;HOP],condition:[f32;CONDITION])->Result<()> {
        if self.failed {return Err("Vocoder worker is terminal; prepare again".into());}
        if self.pending.is_some() {return Err("Vocoder worker already has an in-flight hop".into());}
        let sequence=self.next;
        let next=sequence.checked_add(1).ok_or("Vocoder worker ordinal overflow")?;
        let Some(sender)=&self.requests else {return Err("Vocoder worker is closed".into());};
        match sender.try_send(Job {sequence,signal,condition}) {
            Ok(())=>{self.pending=Some(sequence);self.next=next;self.current=false;Ok(())},
            Err(TrySendError::Full(_))=>{self.cancel();Err("Vocoder worker request queue unexpectedly full".into())},
            Err(TrySendError::Disconnected(_))=>{self.cancel();Err("Vocoder worker disconnected".into())},
        }
    }
    fn finish(&mut self,output:&mut Vec<f32>)->Result<()> {
        if self.failed {return Err("Vocoder worker is terminal; prepare again".into());}
        let expected=self.pending.ok_or("No pending vocoder worker hop")?;
        let packet=match self.results.recv_timeout(WAIT) {
            Ok(packet)=>packet,
            Err(_)=>{self.cancel();return Err("Vocoder worker completion unavailable; prepare again".into());},
        };
        self.pending=None;
        if packet.sequence!=expected || packet.len>HOP {
            self.cancel();return Err("Vocoder worker completion ordinal/extent mismatch".into());
        }

        if packet.error.is_none() {
            if let Some(progress)=packet.progress {self.progress=progress;self.current=true;}
        }
        if let Some(error)=packet.error {self.cancel();return Err(error);}
        output.try_reserve(packet.len).map_err(|e|e.to_string())?;
        output.extend_from_slice(&packet.samples[..packet.len]);
        Ok(())
    }

    fn cancel(&mut self) {
        self.failed=true;self.current=false;
        self.cancel.store(true,Ordering::Release);
        self.requests.take();
    }
}
impl<P:Copy+Send+'static> Drop for PersistentWorker<P> {
    fn drop(&mut self) {
        self.cancel();

        if let Some(thread)=self.thread.take() {let _=thread.join();}
    }
}

#[cfg(target_arch="x86_64")]
struct WorkerFp {saved:u32}
#[cfg(target_arch="x86_64")]
impl WorkerFp {
    fn enter()->Self {

        unsafe {let saved=std::arch::x86_64::_mm_getcsr();std::arch::x86_64::_mm_setcsr(0x1f80);Self {saved}}
    }
}
#[cfg(target_arch="x86_64")]
impl Drop for WorkerFp {
    fn drop(&mut self) {

        unsafe {std::arch::x86_64::_mm_setcsr(self.saved);}
    }
}
#[cfg(not(target_arch="x86_64"))]
struct WorkerFp;
#[cfg(not(target_arch="x86_64"))]
impl WorkerFp {fn enter()->Self {Self}}

pub(super) enum VocoderExecution {
    Serial(VocoderStream),
    Worker(PersistentWorker<VocoderStartupProgress>),
}
impl VocoderExecution {
    pub(super) fn new(stream:VocoderStream)->Result<Self> {
        Ok(Self::Serial(stream))
    }
    pub(super) fn is_worker(&self)->bool {matches!(self,Self::Worker(_))}
    pub(super) fn has_pending(&self)->bool {matches!(self,Self::Worker(worker) if worker.pending.is_some())}
    pub(super) fn mode(&self)->&'static str {if self.is_worker() {"worker"} else {"serial"}}
    pub(super) fn progress_current(&self)->bool {match self {Self::Serial(_)=>true,Self::Worker(w)=>w.current}}
    pub(super) fn startup_progress(&self)->VocoderStartupProgress {
        match self {Self::Serial(stream)=>stream.startup_progress(),Self::Worker(w)=>w.progress}
    }
    pub(super) fn prefetch(&mut self,signal:[f32;HOP],condition:[f32;CONDITION])->Result<()> {
        match self {Self::Worker(w)=>w.submit(signal,condition),Self::Serial(_)=>Err("Cannot prefetch serial vocoder".into())}
    }
    pub(super) fn finish(&mut self,output:&mut Vec<f32>)->Result<()> {
        match self {Self::Worker(w)=>w.finish(output),Self::Serial(_)=>Err("No serial vocoder completion".into())}
    }
    pub(super) fn process(&mut self,signal:&[f32;HOP],condition:&[f32;CONDITION],
        output:&mut Vec<f32>)->Result<()> {
        match self {
            Self::Serial(stream)=>stream.process(signal,condition,output),
            Self::Worker(w)=>{w.submit(*signal,*condition)?;w.finish(output)},
        }
    }
    pub(super) fn cancel_pending(&mut self) {if let Self::Worker(w)=self {w.cancel();}}
}

