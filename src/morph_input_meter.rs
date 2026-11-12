use std::{sync::{Arc,atomic::{AtomicBool,AtomicU32,AtomicU64,Ordering::SeqCst}},
    time::{Duration,Instant}};
use super::input_gain::InputGain;

const MAX_AGE:Duration=Duration::from_millis(100);
const MARKERS:usize=64;

struct Mailbox {
    request:AtomicU64,guard:AtomicU64,sample_request:AtomicU64,ordinal:AtomicU64,
    frames:AtomicU64,pitch:AtomicU64,pitch_ordinal:AtomicU64,envelope:AtomicU32,exhausted:AtomicBool,closed:AtomicBool,
}

pub(crate) struct Writer {shared:Arc<Mailbox>,envelope:f32,ordinal:u64}

pub(crate) struct Query {shared:Arc<Mailbox>,markers:[Option<(u64,Instant)>;MARKERS],next:usize,
    pitch_ordinal:u64,pitch_seen:Option<Instant>}
pub(crate) struct Pending {request:u64,frames:u64}
pub(crate) struct Observation {pub level:f32,pub pitch:crate::pitch_display::Observation}

pub(crate) fn channel()->(Query,Writer) {
    let shared=Arc::new(Mailbox {request:AtomicU64::new(1),guard:AtomicU64::new(0),
        sample_request:AtomicU64::new(0),ordinal:AtomicU64::new(0),frames:AtomicU64::new(0),
        pitch:AtomicU64::new(0),pitch_ordinal:AtomicU64::new(0),envelope:AtomicU32::new(0),exhausted:AtomicBool::new(false),closed:AtomicBool::new(false)});
    (Query {shared:shared.clone(),markers:[None;MARKERS],next:0,pitch_ordinal:0,pitch_seen:None},
     Writer {shared,envelope:0.,ordinal:0})
}

impl Writer {

    pub(crate) fn begin_block(&mut self,input:&[f32],gain:&InputGain)->Option<Pending> {
        let request=self.shared.request.load(SeqCst);
        self.envelope=gain.meter_block(input,self.envelope)?;
        if request==0 {return None;}
        Some(Pending {request,frames:input.len() as u64})
    }

    pub(crate) fn begin_interleaved_block(&mut self,input:&[f32],channels:usize,
        selected:usize,gain:&InputGain)->Option<Pending> {
        if channels==0 || selected>=channels || input.is_empty() || input.len()%channels!=0 {return None;}
        let request=self.shared.request.load(SeqCst);
        let mut peak_bits=0u32;
        for frame in input.chunks_exact(channels) {
            let magnitude=frame[selected].to_bits() & 0x7fff_ffff;
            if magnitude>=0x7f80_0000 {return None;}
            peak_bits=peak_bits.max(magnitude);
        }
        self.begin_peak_block(f32::from_bits(peak_bits),input.len()/channels,request,gain)
    }

    pub(crate) fn begin_silent_block(&mut self,frames:usize,gain:&InputGain)->Option<Pending> {
        if frames==0 {return None;}
        let request=self.shared.request.load(SeqCst);
        self.begin_peak_block(0.,frames,request,gain)
    }
    fn begin_peak_block(&mut self,peak:f32,frames:usize,request:u64,gain:&InputGain)->Option<Pending> {
        self.envelope=gain.meter_block(std::slice::from_ref(&peak),self.envelope)?;
        if request==0 {return None;}
        Some(Pending {request,frames:frames as u64})
    }

    pub(crate) fn finish_block(&mut self,pending:Option<Pending>) {self.finish_block_with_pitch(pending,Default::default())}
    pub(crate) fn finish_block_with_pitch(&mut self,pending:Option<Pending>,pitch:crate::pitch_display::Observation) {
        let Some(pending)=pending else {return;};
        let shared=&self.shared;
        if shared.exhausted.load(SeqCst) {return;}
        let before=shared.guard.load(SeqCst);
        if self.ordinal==u64::MAX || before>u64::MAX-2 {
            shared.exhausted.store(true,SeqCst);return;
        }
        self.ordinal+=1;
        shared.guard.store(before+1,SeqCst);
        shared.sample_request.store(pending.request,SeqCst);
        shared.ordinal.store(self.ordinal,SeqCst);
        shared.frames.store(pending.frames,SeqCst);
        shared.envelope.store(self.envelope.to_bits(),SeqCst);
        shared.pitch.store(pitch.word,SeqCst);
        shared.pitch_ordinal.store(pitch.ordinal,SeqCst);
        shared.guard.store(before+2,SeqCst);
    }
}
impl Drop for Writer {
    fn drop(&mut self) {self.shared.closed.store(true,SeqCst);}
}

impl Query {

    pub(crate) fn poll(&mut self)->Option<f32> {self.poll_observation().map(|v|v.level)}
    pub(crate) fn poll_observation(&mut self)->Option<Observation> {
        let shared=&self.shared;
        if shared.closed.load(SeqCst) || shared.exhausted.load(SeqCst) {return None;}
        let now=Instant::now();
        let mut result=None;
        for _ in 0..3 {
            let before=shared.guard.load(SeqCst);
            if before==0 || before&1!=0 {continue;}
            let request=shared.sample_request.load(SeqCst);
            let ordinal=shared.ordinal.load(SeqCst);
            let frames=shared.frames.load(SeqCst);
            let level=f32::from_bits(shared.envelope.load(SeqCst));
            let pitch=shared.pitch.load(SeqCst);
            let pitch_ordinal=shared.pitch_ordinal.load(SeqCst);
            if before!=shared.guard.load(SeqCst) {continue;}
            if ordinal!=0 && frames!=0 && level.is_finite() && level>=0. {
                let issued=self.markers.iter().find_map(|entry|match entry {
                    Some((marker,issued)) if *marker==request=>Some(*issued),_=>None,
                });
                if issued.is_some_and(|issued|now.saturating_duration_since(issued)<=MAX_AGE) {
                    if pitch_ordinal>self.pitch_ordinal {
                        self.pitch_ordinal=pitch_ordinal;self.pitch_seen=issued;
                    }
                    let current=pitch_ordinal!=0 && pitch_ordinal==self.pitch_ordinal
                        && self.pitch_seen.is_some_and(|seen|now.saturating_duration_since(seen)<=MAX_AGE)
                        && crate::pitch_display::valid(pitch);
                    result=Some(Observation {level,pitch:if current {
                        crate::pitch_display::Observation{word:pitch,ordinal:pitch_ordinal}
                    } else {Default::default()}});
                }
            }
            break;
        }
        if shared.closed.load(SeqCst) || shared.exhausted.load(SeqCst) {return None;}

        let before=shared.request.load(SeqCst);
        if before==0 || before==u64::MAX {
            shared.exhausted.store(true,SeqCst);return None;
        }
        let next=before+1;
        shared.request.store(next,SeqCst);
        self.markers[self.next]=Some((next,now));
        self.next=(self.next+1)%MARKERS;
        result
    }
}
