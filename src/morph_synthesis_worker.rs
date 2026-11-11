use std::{panic::{catch_unwind,AssertUnwindSafe},sync::{Arc,atomic::{AtomicBool,Ordering},
    mpsc::{self,Receiver,SyncSender,TrySendError}},thread::{self,JoinHandle},time::Duration};
use crate::{Result,audio_scheduling::SchedulingSnapshot};
use super::super::synthesis::{SynthesisBackend,SynthesisStatistics,SynthesisStartupProgress};

const WAIT:Duration=Duration::from_millis(100);

#[derive(Clone,Copy,PartialEq,Eq)]
pub(crate) enum Selection {Serial,Worker,Auto}
impl Selection {
    pub(crate) fn default_auto()->Result<Self> {Self::Auto.resolve_current()}

    fn requested_for_instance(multithread:bool)->Self {
        if multithread {Self::Auto} else {Self::Serial}
    }
    pub(crate) fn for_instance(multithread:bool)->Result<Self> {
        Self::requested_for_instance(multithread).resolve_current()
    }
    fn resolve_current(self)->Result<Self> {
        if self==Self::Serial {return Ok(Self::Serial);}
        let cpus=std::thread::available_parallelism().ok().map(|v|v.get());
        self.resolve(false,cfg!(all(windows,target_arch="x86_64")),cpus)
    }
    pub(super) fn avoid_nested(self,inner_worker:bool)->Result<Self> {
        match (self,inner_worker) {
            (Self::Worker,true)=>Err("Synthesis worker refuses nested vocoder worker".into()),
            (Self::Auto,true)=>Ok(Self::Serial),
            (value,_)=>Ok(value),
        }
    }
    fn resolve(self,inner_worker:bool,supported:bool,cpus:Option<usize>)->Result<Self> {
        let selected=self.avoid_nested(inner_worker)?;
        match selected {
            Self::Serial=>Ok(Self::Serial),
            Self::Auto if !supported || !cpus.is_some_and(|n|n>=2)=>Ok(Self::Serial),
            Self::Worker if !supported=>Err("Synthesis worker requires Windows x86-64 scheduling/FP support".into()),

            value=>Ok(value),
        }
    }
}

pub(super) struct Row {pub ordinal:u64,pub raw:[f32;72],pub normalized:[f32;72]}
#[derive(Clone,Copy)]
pub(super) struct Progress {
    pub statistics:SynthesisStatistics,pub startup:SynthesisStartupProgress,
}
trait Processor:Send+'static {
    type Progress:Copy+Send+'static;
    fn run(&mut self,row:&Row,output:&mut Vec<f32>)->Result<()>;
    fn progress(&self)->Self::Progress;
}
impl Processor for SynthesisBackend {
    type Progress=Progress;
    fn run(&mut self,row:&Row,output:&mut Vec<f32>)->Result<()> {
        self.process(&row.raw,&row.normalized,output)
    }
    fn progress(&self)->Progress {
        Progress {statistics:self.statistics(),startup:self.startup_progress()}
    }
}
struct Job {sequence:u64,trigger:u64,first:u64,end:u64,rows:Vec<Row>,samples:Vec<f32>}
struct Packet<P> {job:Job,progress:Option<P>,error:Option<String>}
#[derive(Clone,Copy)]
pub(super) struct Finished {pub trigger:u64,pub first:u64,pub end:u64}
#[derive(Clone,Copy)]
struct Pending {sequence:u64,trigger:u64,first:u64,end:u64}

struct Worker<P:Copy+Send+'static> {
    requests:Option<SyncSender<Job>>,results:Receiver<Packet<P>>,thread:Option<JoinHandle<()>>,
    cancel:Arc<AtomicBool>,pending:Option<Pending>,next_sequence:u64,next_row:u64,
    spare_rows:Vec<Row>,spare_samples:Vec<f32>,progress:P,current:bool,failed:bool,
    scheduling:Option<SchedulingSnapshot>,
}
impl<P:Copy+Send+'static> Worker<P> {

    fn try_spawn_with<T,F,G>(processor:T,register:F)->std::result::Result<Self,StartFailure<T>>
    where T:Processor<Progress=P>,F:FnOnce()->Result<(G,Option<SchedulingSnapshot>)>+Send+'static,G:'static {
        let progress=processor.progress();
        let (requests,jobs)=mpsc::sync_channel::<Job>(1);
        let (completed,results)=mpsc::sync_channel(1);
        let (started,ready)=mpsc::sync_channel(1);
        let (send_owner,receive_owner)=mpsc::sync_channel::<T>(1);
        let cancel=Arc::new(AtomicBool::new(false));
        let worker_cancel=cancel.clone();
        let launched=thread::Builder::new().name("Rocoflex synthesis".into()).spawn(move || {

            let (_registration,status)=match register() {
                Ok(registered)=>registered,
                Err(error)=>{let _=started.try_send(Err(error));return;},
            };
            if started.try_send(Ok(status)).is_err() {return;}
            let mut processor=match receive_owner.recv() {Ok(owner)=>owner,Err(_)=>return};
            if worker_cancel.load(Ordering::Acquire) {return;}
            while let Ok(mut job)=jobs.recv() {
                if worker_cancel.load(Ordering::Acquire) {break;}

                job.samples.clear();
                let outcome=catch_unwind(AssertUnwindSafe(||->Result<T::Progress> {
                    let _fp=WorkerFp::enter();
                    for row in &job.rows {processor.run(row,&mut job.samples)?;}
                    if job.samples.iter().any(|v|!v.is_finite()) {
                        return Err("Synthesis worker produced nonfinite samples".into());
                    }
                    Ok(processor.progress())
                }));
                let (progress,error)=match outcome {
                    Ok(Ok(progress))=>(Some(progress),None),
                    Ok(Err(error))=>(None,Some(error)),
                    Err(_)=>(None,Some("Synthesis worker panicked; prepare again".into())),
                };
                let stop=error.is_some();
                if stop {job.samples.clear();}

                if completed.try_send(Packet {job,progress,error}).is_err() || stop {break;}
            }
        });
        let thread=match launched {
            Ok(thread)=>thread,
            Err(error)=>return Err(StartFailure {processor,error:format!("Cannot start synthesis worker: {error}")}),
        };
        let scheduling=match ready.recv_timeout(WAIT) {
            Ok(Ok(status))=>status,
            outcome=>{
                let error=match outcome {Ok(Err(error))=>error,_=>"Synthesis registration startup unavailable".into()};
                cancel.store(true,Ordering::Release);drop(send_owner);drop(requests);
                let _=thread.join();
                return Err(StartFailure {processor,error});
            },
        };

        match send_owner.try_send(processor) {
            Ok(())=>{},
            Err(TrySendError::Full(processor))|Err(TrySendError::Disconnected(processor))=>{
                cancel.store(true,Ordering::Release);drop(send_owner);drop(requests);let _=thread.join();
                return Err(StartFailure {processor,error:"Synthesis owner transfer unavailable".into()});
            },
        }
        drop(send_owner);
        Ok(Self {requests:Some(requests),results,thread:Some(thread),cancel,pending:None,
            next_sequence:0,next_row:0,spare_rows:Vec::new(),spare_samples:Vec::new(),
            progress,current:true,failed:false,scheduling})
    }
    fn submit(&mut self,trigger:u64,rows:&mut Vec<Row>,accepted:u64)->Result<()> {
        if self.failed {return Err("Synthesis worker is terminal; prepare again".into());}
        if self.pending.is_some() {return Err("Synthesis batch already in flight".into());}
        if rows.is_empty() {return Ok(());}
        let sequence=self.next_sequence;
        let next=sequence.checked_add(1).ok_or("Synthesis job ordinal overflow")?;
        let mut end=self.next_row;
        for row in rows.iter() {
            if row.ordinal!=end || row.raw.iter().chain(&row.normalized).any(|v|!v.is_finite()) {
                return Err("Synthesis batch ordinal/finite validation failed".into());
            }
            end=end.checked_add(1).ok_or("Synthesis decoded ordinal overflow")?;
        }

        if end>accepted {return Err("Synthesis batch exceeds accepted decoder debt".into());}
        let pending=Pending {sequence,trigger,first:self.next_row,end};
        let mut spare=std::mem::take(&mut self.spare_rows);spare.clear();
        let owned=std::mem::replace(rows,spare);
        let samples=std::mem::take(&mut self.spare_samples);
        let job=Job {sequence,trigger,first:pending.first,end,rows:owned,samples};
        let Some(sender)=&self.requests else {self.cancel();return Err("Synthesis worker closed".into());};
        match sender.try_send(job) {
            Ok(())=>{self.pending=Some(pending);self.next_sequence=next;self.next_row=end;self.current=false;Ok(())},
            Err(TrySendError::Full(_))=>{self.cancel();Err("Synthesis request slot unexpectedly full".into())},
            Err(TrySendError::Disconnected(_))=>{self.cancel();Err("Synthesis worker disconnected".into())},
        }
    }
    fn finish(&mut self,output:&mut Vec<f32>)->Result<Option<Finished>> {
        if self.failed {return Err("Synthesis worker is terminal; prepare again".into());}
        let Some(expected)=self.pending else {return Ok(None);};
        let packet=match self.results.recv_timeout(WAIT) {
            Ok(packet)=>packet,
            Err(_)=>{self.cancel();return Err("Synthesis completion unavailable; prepare again".into());},
        };
        self.pending=None;
        let mut job=packet.job;
        if job.sequence!=expected.sequence || job.trigger!=expected.trigger || job.first!=expected.first
            || job.end!=expected.end || job.rows.len() as u64!=expected.end-expected.first {
            self.cancel();return Err("Synthesis completion identity mismatch".into());
        }
        if let Some(error)=packet.error {self.cancel();return Err(error);}
        let Some(progress)=packet.progress else {self.cancel();return Err("Missing successful synthesis progress".into());};
        if job.samples.iter().any(|v|!v.is_finite()) {self.cancel();return Err("Nonfinite synthesis completion".into());}
        if let Err(error)=output.try_reserve(job.samples.len()) {self.cancel();return Err(error.to_string());}
        output.extend_from_slice(&job.samples);
        job.samples.clear();job.rows.clear();
        self.spare_samples=job.samples;self.spare_rows=job.rows;
        self.progress=progress;self.current=true;
        Ok(Some(Finished {trigger:expected.trigger,first:expected.first,end:expected.end}))
    }
    fn cancel(&mut self) {
        self.failed=true;self.current=false;
        self.cancel.store(true,Ordering::Release);self.requests.take();
    }
}
impl<P:Copy+Send+'static> Drop for Worker<P> {
    fn drop(&mut self) {
        self.cancel();

        if let Some(thread)=self.thread.take() {let _=thread.join();}
    }
}

struct StartFailure<T> {processor:T,error:String}
enum PreparedOwner<T:Processor> {Serial(T),Worker(Worker<T::Progress>)}
fn prepare_owner<T,F,G>(processor:T,selection:Selection,register:F)->Result<PreparedOwner<T>>
where T:Processor,F:FnOnce()->Result<(G,Option<SchedulingSnapshot>)>+Send+'static,G:'static {
    if selection==Selection::Serial {return Ok(PreparedOwner::Serial(processor));}
    match Worker::try_spawn_with(processor,register) {
        Ok(worker)=>Ok(PreparedOwner::Worker(worker)),
        Err(failure) if selection==Selection::Auto=>Ok(PreparedOwner::Serial(failure.processor)),
        Err(failure)=>Err(failure.error),
    }
}
fn register_audio()->Result<(crate::audio_scheduling::Registration,Option<SchedulingSnapshot>)> {
    let mut registration=crate::audio_scheduling::Registration::new();
    registration.register();let status=registration.snapshot();
    if !status.registration_succeeded {
        return Err(format!("Synthesis Audio MMCSS registration failed: {:?}",status.registration_hresult));
    }
    Ok((registration,Some(status)))
}
pub(super) enum Prepared {Serial(SynthesisBackend),Worker(Pipeline)}
pub(super) struct Pipeline {worker:Worker<Progress>}
impl Pipeline {
    pub(super) fn prepare(synthesis:SynthesisBackend,selection:Selection)->Result<Prepared> {
        match prepare_owner(synthesis,selection,register_audio)? {
            PreparedOwner::Serial(owner)=>Ok(Prepared::Serial(owner)),
            PreparedOwner::Worker(worker)=>Ok(Prepared::Worker(Self {worker})),
        }
    }
    pub(super) fn submit(&mut self,trigger:u64,rows:&mut Vec<Row>,accepted:u64)->Result<()> {self.worker.submit(trigger,rows,accepted)}
    pub(super) fn finish(&mut self,output:&mut Vec<f32>)->Result<Option<Finished>> {self.worker.finish(output)}
    pub(super) fn has_pending(&self)->bool {self.worker.pending.is_some()}
    pub(super) fn cancel(&mut self) {self.worker.cancel();}

    pub(super) fn scheduling_snapshot(&self)->Option<SchedulingSnapshot> {self.worker.scheduling}
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
    fn drop(&mut self) {unsafe {std::arch::x86_64::_mm_setcsr(self.saved);}}
}
#[cfg(not(target_arch="x86_64"))]
struct WorkerFp;
#[cfg(not(target_arch="x86_64"))]
impl WorkerFp {fn enter()->Self {Self}}

