use std::collections::VecDeque;
use crate::ltv_block::SourceLtvBlockConfig;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum LtvRowOrigin {ResetZero,Input(u64),FlushZero}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct LtvBlockOwnership {
    pub time_rows:Vec<LtvRowOrigin>,pub coefficient:LtvRowOrigin,
    pub phase_before:i64,pub visible_sample_start:usize,pub visible_sample_count:usize,
    pub completes_output_row:bool,
}
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct LtvBatchOwnership {pub blocks:Vec<LtvBlockOwnership>,pub output_rows:usize,pub output_status:u8}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum LtvHistoryError {AlreadyEnded,StatusMismatch,CounterOverflow,InvalidQueueState}

pub struct SourceLtvHistory {
    config:SourceLtvBlockConfig,window_rows:usize,time:VecDeque<LtvRowOrigin>,coefficients:VecDeque<LtvRowOrigin>,
    received:u64,emitted:u64,phase:i64,partial:usize,ended:bool,
}
impl SourceLtvHistory {
    pub fn from_kernel_reset(config:SourceLtvBlockConfig)->Self {
        let window_rows=config.window_length()/config.hop();
        let mut time=VecDeque::new();
        for _ in 0..window_rows/2 {time.push_back(LtvRowOrigin::ResetZero);}
        Self{config,window_rows,time,coefficients:VecDeque::new(),received:0,emitted:0,
          phase:-(config.transform_length() as i64/2),partial:0,ended:false}
    }
    pub fn pending_partial_samples(&self)->usize {self.partial}
    pub fn received_rows(&self)->u64 {self.received}
    pub fn emitted_rows(&self)->u64 {self.emitted}
    fn block(&mut self)->Result<LtvBlockOwnership,LtvHistoryError> {
        if self.time.len()!=self.window_rows {return Err(LtvHistoryError::InvalidQueueState);}
        let coefficient=self.coefficients.pop_front().ok_or(LtvHistoryError::InvalidQueueState)?;
        let time_rows=self.time.iter().copied().collect();self.time.pop_front();
        let phase_before=self.phase;
        let phase_after=self.phase.checked_add(self.config.hop() as i64).ok_or(LtvHistoryError::CounterOverflow)?;
        let visible_sample_count=(phase_after.max(0)-self.phase.max(0)) as usize;
        let visible_sample_start=(self.phase.max(0)-self.phase) as usize;
        self.partial+=visible_sample_count;
        let completes_output_row=self.partial>=self.config.hop();
        if completes_output_row {self.partial-=self.config.hop();self.emitted+=1;}
        self.phase=phase_after;
        Ok(LtvBlockOwnership{time_rows,coefficient,phase_before,
          visible_sample_start:visible_sample_start.min(self.config.hop()),visible_sample_count,completes_output_row})
    }

    pub fn plan_paired_batch(&mut self,rows:usize,status_a:u8,status_b:u8)
      ->Result<LtvBatchOwnership,LtvHistoryError> {
        if self.ended {return Err(LtvHistoryError::AlreadyEnded);}
        if status_a!=status_b {return Err(LtvHistoryError::StatusMismatch);}
        let new_received=self.received.checked_add(rows as u64).ok_or(LtvHistoryError::CounterOverflow)?;
        if new_received>i64::MAX as u64 {return Err(LtvHistoryError::CounterOverflow);}
        self.ended=status_a!=0;
        let emitted_before=self.emitted;let mut blocks=Vec::new();
        for _ in 0..rows {
            let origin=LtvRowOrigin::Input(self.received);
            self.time.push_back(origin);self.coefficients.push_back(origin);
            if self.time.len()==self.window_rows {blocks.push(self.block()?);}
            self.received+=1;
        }
        if self.ended {
            while self.received!=self.emitted {
                if self.coefficients.is_empty() {self.coefficients.push_back(LtvRowOrigin::FlushZero);}
                self.time.push_back(LtvRowOrigin::FlushZero);
                if self.time.len()==self.window_rows {blocks.push(self.block()?);}
            }
        }
        Ok(LtvBatchOwnership{blocks,output_rows:(self.emitted-emitted_before) as usize,output_status:u8::from(self.ended)})
    }
}
